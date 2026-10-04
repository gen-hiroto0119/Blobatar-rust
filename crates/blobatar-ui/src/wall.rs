use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use blobatar_core::{Avatar, Expression, Options};
use blobatar_gpui::{
    Blobatar, Drawing,
    gpui::{
        self, AnyElement, Bounds, Context, Entity, KeyDownEvent, MouseButton, MouseDownEvent,
        MouseMoveEvent, MouseUpEvent, Pixels, Point, Render, ScrollWheelEvent, SharedString,
        Subscription, Window, canvas, div, point, prelude::*, px, quad, rgb, size,
    },
};
use blobatar_wall::{
    Cell, PlaceInput, Placement, SQLiteStore, WallError, WallStore, cell_at, check_name,
    chunks_covering, day_of, hash_identity, hash_token, is_token, new_token, region_of,
};

use crate::{text_input::TextInput, wall_camera::Camera};

struct LocalWall {
    store: Arc<dyn WallStore>,
    token: String,
    path: PathBuf,
}

impl LocalWall {
    fn open() -> Result<Self, String> {
        let path = match std::env::var_os("BLOBATAR_WALL_DB") {
            Some(path) => PathBuf::from(path),
            None => PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?)
                .join(".blobatar/wall.sqlite3"),
        };
        if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let identity_path = path.with_extension("desktop-identity");
        let mut create = OpenOptions::new();
        create.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            create.mode(0o600);
        }
        let token = match create.open(&identity_path) {
            Ok(mut file) => {
                let token = new_token();
                file.write_all(token.as_bytes())
                    .map_err(|error| error.to_string())?;
                file.sync_all().map_err(|error| error.to_string())?;
                token
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut token = String::new();
                std::fs::File::open(&identity_path)
                    .map_err(|error| error.to_string())?
                    .take(65)
                    .read_to_string(&mut token)
                    .map_err(|error| error.to_string())?;
                if !is_token(&token) {
                    return Err("Invalid local identity file".into());
                }
                token
            }
            Err(error) => return Err(error.to_string()),
        };
        let store = Arc::new(SQLiteStore::open(&path).map_err(|error| error.to_string())?);
        Ok(Self { store, token, path })
    }
}

struct Snapshot {
    cells: HashMap<Cell, Placement>,
    total: i64,
}

fn snapshot(store: &dyn WallStore, bounds: (i32, i32, i32, i32)) -> Result<Snapshot, WallError> {
    let mut cells = HashMap::new();
    let mut total = 0;
    let mut regions = HashSet::new();
    let mut versions = HashSet::new();
    let chunks = chunks_covering(bounds.0, bounds.1, bounds.2, bounds.3);
    for chunk in &chunks {
        let region = region_of(*chunk);
        if regions.insert(region) {
            let index = store.region(region)?;
            total = index.placements;
            versions.extend(
                index
                    .chunks
                    .into_iter()
                    .filter(|state| state.count > 0)
                    .map(|state| state.key),
            );
        }
    }
    for chunk in chunks {
        if !versions.contains(&blobatar_wall::chunk_key(chunk)) {
            continue;
        }
        for placement in store.chunk(chunk)?.cells {
            let cell = cell_at(chunk, placement.index);
            cells.insert(
                cell,
                Placement {
                    x: cell.x,
                    y: cell.y,
                    seed: placement.seed,
                    expression: placement.expression,
                    at: placement.at,
                },
            );
        }
    }
    Ok(Snapshot { cells, total })
}

pub struct Wall {
    local: Option<LocalWall>,
    camera: Camera,
    bounds: Bounds<Pixels>,
    cells: HashMap<Cell, Placement>,
    drawings: HashMap<Cell, Arc<Drawing>>,
    total: i64,
    selected: Cell,
    expression: Expression,
    name: Entity<TextInput>,
    _name_subscription: Subscription,
    ghost: Arc<Drawing>,
    status: String,
    drag: Option<(Point<Pixels>, Point<Pixels>, bool)>,
    loading: bool,
    reload: bool,
    writing: bool,
}

impl Wall {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| TextInput::new("ひろと", cx));
        let subscription = cx.observe(&name, |this: &mut Self, _, cx| this.refresh_ghost(cx));
        let mut wall = Self {
            local: None,
            camera: Camera::default(),
            bounds: Bounds::default(),
            cells: HashMap::new(),
            drawings: HashMap::new(),
            total: 0,
            selected: Cell { x: 0, y: 0 },
            expression: Expression::Idle,
            name,
            _name_subscription: subscription,
            ghost: drawing("ひろと", Expression::Idle),
            status: "ローカル壁を読込中 / Opening local wall".into(),
            drag: None,
            loading: false,
            reload: false,
            writing: false,
        };
        wall.open(cx);
        wall
    }

    fn open(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let opened = cx
                .background_executor()
                .spawn(async { LocalWall::open() })
                .await;
            let _ = this.update(cx, |this, cx| {
                match opened {
                    Ok(local) => {
                        this.status = format!("ローカル保存 / Local: {}", local.path.display());
                        this.local = Some(local);
                        this.load(cx);
                    }
                    Err(error) => this.status = format!("読込失敗 / Open failed: {error}"),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn viewport(&self) -> (f64, f64) {
        (
            f64::from(f32::from(self.bounds.size.width)),
            f64::from(f32::from(self.bounds.size.height)),
        )
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let Some(local) = &self.local else {
            return;
        };
        if self.loading {
            self.reload = true;
            return;
        }
        let store = local.store.clone();
        let bounds = self
            .camera
            .visible_box(self.viewport(), blobatar_wall::REACH);
        self.loading = true;
        self.reload = false;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { snapshot(store.as_ref(), bounds) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.loading = false;
                if this.reload {
                    this.load(cx);
                    return;
                }
                match result {
                    Ok(snapshot) => {
                        this.drawings
                            .retain(|cell, _| this.cells.get(cell) == snapshot.cells.get(cell));
                        this.cells = snapshot.cells;
                        this.total = snapshot.total;
                    }
                    Err(error) => this.status = format!("読込失敗 / Read failed: {error}"),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn refresh_ghost(&mut self, cx: &mut Context<Self>) {
        let name = self.name.read(cx).text();
        let seed = check_name(Some(name), None).unwrap_or_else(|_| name.to_owned());
        self.ghost = drawing(
            if seed.is_empty() { "blobatar" } else { &seed },
            self.expression,
        );
        cx.notify();
    }

    fn zoom(&mut self, factor: f64, at: (f64, f64), cx: &mut Context<Self>) {
        self.camera.zoom_at(self.viewport(), at, factor);
        self.load(cx);
        cx.notify();
    }

    fn choose(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let relative = position - self.bounds.origin;
        let (x, y) = self.camera.cell_under(
            self.viewport(),
            (
                f64::from(f32::from(relative.x)),
                f64::from(f32::from(relative.y)),
            ),
        );
        self.selected = Cell { x, y };
        cx.notify();
    }

    fn place(&mut self, cx: &mut Context<Self>) {
        let Some(local) = &self.local else {
            return;
        };
        if self.writing {
            return;
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let input = PlaceInput {
            cell: self.selected,
            seed: self.name.read(cx).text().into(),
            expression: self.expression.name().into(),
            now,
            identity: hash_identity("desktop-local", &day_of(now), &local.token),
            token: hash_token(&local.token),
        };
        let store = local.store.clone();
        self.writing = true;
        self.status = "投稿中 / Placing".into();
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async move { store.place(input) }).await;
            let _ = this.update(cx, |this, cx| {
                this.writing = false;
                match result {
                    Ok(placed) => {
                        this.status = format!("投稿しました / Placed: {} ({}, {})", placed.placement.seed, placed.placement.x, placed.placement.y);
                        this.load(cx);
                    }
                    Err(WallError::Unplaceable { nearest: Some(cell) }) => {
                        this.selected = cell;
                        this.camera.x = f64::from(cell.x);
                        this.camera.y = f64::from(cell.y);
                        this.status = "投稿できる候補へ移動しました。確認して再投稿 / Moved to suggestion; review and place again".into();
                        this.load(cx);
                    }
                    Err(WallError::Cooldown { until }) => this.status = format!("本日は投稿済み / Daily limit — next UTC midnight: {} (UTC)", day_of(until)),
                    Err(error) => this.status = format!("投稿できません / Not placed: {error}"),
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn find_mine(&mut self, cx: &mut Context<Self>) {
        let Some(local) = &self.local else {
            return;
        };
        let store = local.store.clone();
        let token = hash_token(&local.token);
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { store.mine(&token, 8) })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(cells) => match cells.first() {
                        Some(cell) => {
                            this.selected = Cell {
                                x: cell.x,
                                y: cell.y,
                            };
                            this.camera.x = f64::from(cell.x);
                            this.camera.y = f64::from(cell.y);
                            this.status = format!("自分の最新投稿 / Latest of mine: {}", cell.seed);
                            this.load(cx);
                        }
                        None => {
                            this.status =
                                "自分の投稿はまだありません / No placements for this local identity"
                                    .into()
                        }
                    },
                    Err(error) => this.status = format!("検索失敗 / Find failed: {error}"),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn button(
        &self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        selected: bool,
        cx: &mut Context<Self>,
        action: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let action = Arc::new(action);
        let key_action = action.clone();
        div()
            .id(id.into())
            .focusable()
            .tab_stop(true)
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(if selected { 0x6d91bd } else { 0x323c4c }))
            .bg(rgb(if selected { 0x354a70 } else { 0x222938 }))
            .text_sm()
            .cursor_pointer()
            .focus(|style| style.border_color(rgb(0xf4c76b)))
            .child(label.into())
            .on_click(cx.listener(move |this, _, _, cx| action(this, cx)))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    key_action(this, cx);
                    cx.stop_propagation();
                }
            }))
            .into_any_element()
    }

    fn panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let occupied = self.cells.get(&self.selected);
        div().id("wall-panel").w(px(300.0)).flex_shrink_0().h_full().overflow_y_scroll()
            .p_4().flex().flex_col().gap_3().rounded_lg().bg(rgb(0x181e29))
            .child(div().text_lg().child(format!("セル / Cell {}, {}", self.selected.x, self.selected.y)))
            .when_some(occupied, |panel, cell| panel
                .child(format!("名前 / Name: {}", cell.seed))
                .child(format!("表情 / Expression: {}", cell.expression))
                .child(format!("投稿日 / Placed: {} (UTC)", day_of(cell.at))))
            .when(occupied.is_none(), |panel| panel
                .child("名前 / Name · 最大24文字 / 24 code points")
                .child(self.name.clone())
                .child(div().flex().justify_center().child(Blobatar::from_drawing(self.ghost.clone()).size(128.0)))
                .child(div().flex().flex_wrap().gap_2().children(Expression::ALL.into_iter().map(|expression| {
                    self.button(format!("wall-expression-{}", expression.name()), expression.name(), self.expression == expression, cx,
                        move |this, cx| { this.expression = expression; this.refresh_ghost(cx); })
                })))
                .child(self.button("wall-place", if self.writing { "投稿中 / Placing…" } else { "ここに投稿 / Place here" }, false, cx, Self::place)))
            .child(div().text_xs().child("ローカルidentity · 1日1投稿（UTC） / One placement per UTC day. この壁から外部サービスへは送信しません / No external posts."))
            .into_any_element()
    }

    fn surface(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let camera = self.camera;
        let view = self.viewport();
        let selection = self.selected;
        let weak = cx.entity().downgrade();
        let mut avatars = Vec::new();
        let mut dots = Vec::new();
        let (x0, y0, x1, y1) = camera.visible_box(view, 1);
        self.drawings
            .retain(|cell, _| cell.x >= x0 && cell.x <= x1 && cell.y >= y0 && cell.y <= y1);
        for (cell, placement) in &self.cells {
            if cell.x < x0 || cell.x > x1 || cell.y < y0 || cell.y > y1 {
                continue;
            }
            let (x, y) = camera.to_screen(view, (f64::from(cell.x), f64::from(cell.y)));
            let side = (camera.scale() * 0.85) as f32;
            if camera.zoom < 0.3 {
                dots.push((x as f32, y as f32, side));
                continue;
            }
            let drawing = self
                .drawings
                .entry(*cell)
                .or_insert_with(|| {
                    let expression = Expression::ALL
                        .into_iter()
                        .find(|expression| expression.name() == placement.expression)
                        .unwrap_or_default();
                    drawing(&placement.seed, expression)
                })
                .clone();
            avatars.push(
                div()
                    .absolute()
                    .left(px(x as f32 - side / 2.0))
                    .top(px(y as f32 - side / 2.0))
                    .child(Blobatar::from_drawing(drawing).size(side))
                    .into_any_element(),
            );
        }
        let (sx, sy) = camera.to_screen(view, (f64::from(selection.x), f64::from(selection.y)));
        let side = camera.scale() as f32;
        let ghost = (!self.cells.contains_key(&selection)).then(|| {
            div()
                .absolute()
                .left(px(sx as f32 - side * 0.425))
                .top(px(sy as f32 - side * 0.425))
                .opacity(0.45)
                .child(Blobatar::from_drawing(self.ghost.clone()).size(side * 0.85))
        });
        div()
            .id("wall-surface")
            .focusable()
            .tab_stop(true)
            .flex_1()
            .h_full()
            .min_w_0()
            .relative()
            .overflow_hidden()
            .rounded_lg()
            .bg(rgb(0x10151e))
            .border_1()
            .border_color(rgb(0x323c4c))
            .child(
                canvas(
                    move |bounds, _, cx| {
                        let _ = weak.update(cx, |this, cx| {
                            if this.bounds != bounds {
                                this.bounds = bounds;
                                this.load(cx);
                                cx.notify();
                            }
                        });
                    },
                    move |bounds, _, window, _| {
                        if camera.zoom >= 0.5 {
                            let color = rgb(0x1c2634);
                            for x in x0..=x1 {
                                let (x, _) = camera.to_screen(view, (f64::from(x) - 0.5, 0.0));
                                window.paint_quad(quad(
                                    Bounds {
                                        origin: bounds.origin + point(px(x as f32), px(0.0)),
                                        size: size(px(1.0), bounds.size.height),
                                    },
                                    px(0.0),
                                    color,
                                    px(0.0),
                                    gpui::transparent_black(),
                                    Default::default(),
                                ));
                            }
                            for y in y0..=y1 {
                                let (_, y) = camera.to_screen(view, (0.0, f64::from(y) - 0.5));
                                window.paint_quad(quad(
                                    Bounds {
                                        origin: bounds.origin + point(px(0.0), px(y as f32)),
                                        size: size(bounds.size.width, px(1.0)),
                                    },
                                    px(0.0),
                                    color,
                                    px(0.0),
                                    gpui::transparent_black(),
                                    Default::default(),
                                ));
                            }
                        }
                        for (x, y, width) in dots {
                            window.paint_quad(quad(
                                Bounds {
                                    origin: bounds.origin
                                        + point(px(x - width / 2.0), px(y - width / 2.0)),
                                    size: size(px(width), px(width)),
                                },
                                px(width / 2.0),
                                rgb(0x93aac8),
                                px(0.0),
                                gpui::transparent_black(),
                                Default::default(),
                            ));
                        }
                        window.paint_quad(quad(
                            Bounds {
                                origin: bounds.origin
                                    + point(px(sx as f32 - side / 2.0), px(sy as f32 - side / 2.0)),
                                size: size(px(side), px(side)),
                            },
                            px(6.0),
                            gpui::transparent_black(),
                            px(2.0),
                            rgb(0xf4c76b),
                            Default::default(),
                        ));
                    },
                )
                .absolute()
                .size_full(),
            )
            .children(avatars)
            .children(ghost)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, _| {
                    this.drag = Some((event.position, event.position, false));
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if let Some((start, previous, moved)) = this.drag {
                    if !event.dragging() {
                        this.drag = None;
                        return;
                    }
                    let delta = event.position - previous;
                    let total = event.position - start;
                    let moved = moved || f32::from(total.x).abs() + f32::from(total.y).abs() > 4.0;
                    this.drag = Some((start, event.position, moved));
                    if moved {
                        this.camera
                            .pan(f64::from(f32::from(delta.x)), f64::from(f32::from(delta.y)));
                        this.load(cx);
                        cx.notify();
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    if let Some((_, _, moved)) = this.drag.take()
                        && !moved
                    {
                        this.choose(event.position, cx);
                    }
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.drag = None),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                let delta = event.delta.pixel_delta(px(20.0));
                if event.modifiers.control || event.modifiers.platform {
                    let at = event.position - this.bounds.origin;
                    this.zoom(
                        (-f64::from(f32::from(delta.y)) * 0.01).exp(),
                        (f64::from(f32::from(at.x)), f64::from(f32::from(at.y))),
                        cx,
                    );
                } else {
                    this.camera
                        .pan(f64::from(f32::from(delta.x)), f64::from(f32::from(delta.y)));
                    this.load(cx);
                    cx.notify();
                }
                cx.stop_propagation();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let (dx, dy) = match event.keystroke.key.as_str() {
                    "left" => (-1, 0),
                    "right" => (1, 0),
                    "up" => (0, -1),
                    "down" => (0, 1),
                    _ => return,
                };
                this.selected.x = (this.selected.x + dx).clamp(-1_000_000, 1_000_000);
                this.selected.y = (this.selected.y + dy).clamp(-1_000_000, 1_000_000);
                this.camera.x = f64::from(this.selected.x);
                this.camera.y = f64::from(this.selected.y);
                this.load(cx);
                cx.notify();
                cx.stop_propagation();
            }))
            .into_any_element()
    }
}

impl Render for Wall {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let surface = self.surface(cx);
        div().size_full().p_5().bg(rgb(0x11151d)).text_color(rgb(0xe6edf5)).flex().flex_col().gap_3()
            .child(div().text_2xl().child("Blobatar · 壁 / Wall"))
            .child(div().flex().items_center().gap_2()
                .child(self.button("wall-home", "原点 / Origin", false, cx, |this, cx| {
                    this.camera = Camera::default(); this.selected = Cell { x: 0, y: 0 }; this.load(cx);
                }))
                .child(self.button("wall-out", "−", false, cx, |this, cx| {
                    let view = this.viewport(); this.zoom(1.0 / 1.4, (view.0 / 2.0, view.1 / 2.0), cx);
                }))
                .child(format!("{:.0}%", self.camera.zoom * 100.0))
                .child(self.button("wall-in", "+", false, cx, |this, cx| {
                    let view = this.viewport(); this.zoom(1.4, (view.0 / 2.0, view.1 / 2.0), cx);
                }))
                .child(self.button("wall-mine", "自分を探す / Find mine", false, cx, Self::find_mine))
                .child(self.button("wall-reload", "再読込 / Refresh", false, cx, Self::load))
                .child(format!("{} 体 / placements{}", self.total, if self.loading { " · 読込中 / Loading" } else { "" })))
            .child(div().text_xs().child("ドラッグ・スクロールで移動 / Drag or scroll to pan · Ctrl/⌘+scroll: zoom · 矢印キー / arrows: select"))
            .child(div().text_sm().child(self.status.clone()))
            .child(div().flex().flex_1().min_h_0().gap_4().child(surface).child(self.panel(cx)))
    }
}

fn drawing(seed: &str, expression: Expression) -> Arc<Drawing> {
    let options = Options {
        expression: Some(expression),
        ..Options::default()
    };
    Arc::new(Drawing::new(&Avatar::new(seed, &options), &options))
}
