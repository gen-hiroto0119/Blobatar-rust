use super::*;
use crate::cloud::{self, Page, Saved, User, Work};

impl Editor {
    pub(super) fn init_cloud(&mut self, cx: &mut Context<Self>) {
        self.wall.configured = cloud::configured();
        if let Ok(Some(json)) = cloud::take_draft() {
            match blobatar_export::Settings::from_json(&json) {
                Ok(settings) => self.apply_cloud_settings(settings, cx),
                Err(error) => {
                    self.status = format!("下書きの復元失敗 / Draft restore failed: {error}")
                }
            }
        }
        if self.wall.configured {
            self.reload_cloud(0, cx);
        }
    }

    fn apply_cloud_settings(
        &mut self,
        settings: blobatar_export::Settings,
        cx: &mut Context<Self>,
    ) {
        self.state.settings = settings;
        let name = self.state.settings.name.clone();
        self.name.update(cx, |input, cx| input.set_text(name, cx));
        self.refresh_with_advanced_sync(true, true, cx);
    }

    fn cloud_user(&mut self, user: Option<User>) {
        if self.wall.user.as_ref().map(|user| &user.id) != user.as_ref().map(|user| &user.id) {
            self.wall.works.clear();
            self.wall.selected = None;
            self.wall.delete_pending = None;
        }
        self.wall.user = user;
    }

    fn cloud_error(&mut self, error: String) {
        self.wall.works.clear();
        self.wall.selected = None;
        self.wall.delete_pending = None;
        self.wall.user = None;
        self.status = format!("クラウド操作失敗 / Cloud error: {error}");
    }

    fn reload_cloud(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.wall.busy || !self.wall.configured {
            return;
        }
        self.wall.busy = true;
        self.wall.delete_pending = None;
        cx.spawn(async move |this, cx| {
            let result: Result<Option<Page>, String> = async {
                let user: Option<User> = cloud::json(cloud::session()).await?;
                if user.is_none() {
                    return Ok(None);
                }
                let page: Page = cloud::json(cloud::list(offset)).await?;
                for work in &page.rows {
                    work.settings.validate()?;
                }
                Ok(Some(page))
            }
            .await;
            let _ = this.update(cx, |this, cx| {
                this.wall.busy = false;
                match result {
                    Ok(Some(page)) => {
                        this.cloud_user(Some(page.user));
                        this.wall.works = page.rows;
                        this.wall.more = page.more;
                        this.wall.offset = offset;
                    }
                    Ok(None) => this.cloud_user(None),
                    Err(error) => this.cloud_error(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn cloud_login(&mut self, cx: &mut Context<Self>) {
        if self.wall.busy {
            return;
        }
        let settings = match self.state.settings.to_json() {
            Ok(settings) => settings,
            Err(error) => {
                self.status = error;
                cx.notify();
                return;
            }
        };
        let promise = cloud::login(&settings);
        self.wall.busy = true;
        cx.spawn(async move |this, cx| {
            let result = cloud::resolve(promise).await;
            let _ = this.update(cx, |this, cx| {
                this.wall.busy = false;
                if let Err(error) = result {
                    this.cloud_error(error);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn cloud_logout(&mut self, cx: &mut Context<Self>) {
        if self.wall.busy {
            return;
        }
        self.wall.busy = true;
        cx.spawn(async move |this, cx| {
            let result = cloud::resolve(cloud::logout()).await;
            let _ = this.update(cx, |this, cx| {
                this.wall.busy = false;
                match result {
                    Ok(_) => {
                        this.cloud_user(None);
                        this.status = "ログアウトしました / Signed out".into();
                    }
                    Err(error) => this.cloud_error(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn cloud_save(&mut self, duplicate: bool, cx: &mut Context<Self>) {
        if self.wall.busy || self.busy || self.wall.user.is_none() {
            return;
        }
        let settings = match self.state.settings.to_json() {
            Ok(settings) => settings,
            Err(error) => {
                self.status = error;
                cx.notify();
                return;
            }
        };
        let selected = self.wall.selected.as_ref().filter(|_| !duplicate);
        let promise = cloud::save(
            &settings,
            selected.map_or("", |work| work.id.as_str()),
            selected.map_or("", |work| work.updated_at.as_str()),
        );
        self.wall.busy = true;
        cx.spawn(async move |this, cx| {
            let result: Result<Saved, String> = cloud::json(promise).await;
            let _ = this.update(cx, |this, cx| {
                this.wall.busy = false;
                match result {
                    Ok(saved) => {
                        this.cloud_user(Some(saved.user));
                        this.wall.selected = Some(saved.row);
                        this.status = "保存完了 / Saved to My Wall".into();
                        this.reload_cloud(0, cx);
                    }
                    Err(error) => this.cloud_error(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn cloud_delete(&mut self, work: &Work, cx: &mut Context<Self>) {
        if self.wall.busy {
            return;
        }
        if self.wall.delete_pending.as_ref() != Some(&work.id) {
            self.wall.delete_pending = Some(work.id.clone());
            cx.notify();
            return;
        }
        let id = work.id.clone();
        let promise = cloud::remove(&work.id, &work.updated_at);
        self.wall.busy = true;
        cx.spawn(async move |this, cx| {
            let result = cloud::resolve(promise).await;
            let _ = this.update(cx, |this, cx| {
                this.wall.busy = false;
                this.wall.delete_pending = None;
                match result {
                    Ok(_) => {
                        if this
                            .wall
                            .selected
                            .as_ref()
                            .is_some_and(|work| work.id == id)
                        {
                            this.wall.selected = None;
                        }
                        this.status = "削除完了 / Deleted".into();
                        let offset = if this.wall.works.len() == 1 {
                            this.wall.offset.saturating_sub(cloud::PAGE_SIZE)
                        } else {
                            this.wall.offset
                        };
                        this.reload_cloud(offset, cx);
                    }
                    Err(error) => this.cloud_error(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn wall_toggle(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        Some(
            self.button(
                "my-wall",
                "マイウォール / My Wall",
                false,
                cx,
                |this, _, cx| {
                    this.wall.open = true;
                    this.reload_cloud(0, cx);
                    cx.notify();
                },
            )
            .into_any_element(),
        )
    }

    pub(super) fn cloud_save_controls(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.wall.user.as_ref()?;
        Some(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .flex_shrink_0()
                .child(self.button(
                    "cloud-save",
                    if self.wall.selected.is_some() {
                        "上書き保存 / Update work"
                    } else {
                        "マイウォールに保存 / Save to My Wall"
                    },
                    ButtonStyle::primary(self.wall.busy || self.busy),
                    cx,
                    |this, _, cx| this.cloud_save(false, cx),
                ))
                .when(self.wall.selected.is_some(), |row| {
                    row.child(self.button(
                        "cloud-duplicate",
                        "別の作品として保存 / Save a copy",
                        ButtonStyle::secondary(self.wall.busy || self.busy),
                        cx,
                        |this, _, cx| this.cloud_save(true, cx),
                    ))
                })
                .child(
                    div()
                        .text_size(px(12.0))
                        .child("非公開 · 本人のみ / Private · Only you"),
                )
                .into_any_element(),
        )
    }

    pub(super) fn render_wall(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.appearance.palette();
        div().id("my-wall-page").size_full().flex().flex_col().gap_4().p_6()
            .track_focus(&self.focus_handle).tab_stop(false)
            .on_key_down(|event, window, cx| {
                if event.keystroke.key == "tab" {
                    if event.keystroke.modifiers.shift { window.focus_prev(cx); }
                    else { window.focus_next(cx); }
                    cx.stop_propagation();
                }
            })
            .bg(palette.background).text_color(palette.text).font_family(theme::BODY_FONT)
            .text_size(px(14.0)).line_height(px(22.0))
            .child(div().flex().flex_wrap().gap_3().items_center().justify_between().flex_shrink_0()
                .child(theme::heading("マイウォール / My Wall"))
                .child(self.button("back-editor", "エディターに戻る / Back", false, cx, |this, _, cx| { this.wall.open = false; cx.notify(); })))
            .child(div().flex_shrink_0().text_color(palette.muted).child("作品は本人だけが閲覧できます / Your works are private"))
            .child(div().flex().flex_wrap().gap_2().items_center().flex_shrink_0()
                .when(self.wall.configured, |row| row
                    .child(self.button("refresh-wall", "更新 / Refresh", ButtonStyle::secondary(self.wall.busy), cx, |this, _, cx| this.reload_cloud(0, cx)))
                    .when(self.wall.user.is_none(), |row| row.child(self.button("github-login", "GitHubでログイン / Sign in", ButtonStyle::primary(self.wall.busy), cx, |this, _, cx| this.cloud_login(cx))))
                    .when_some(self.wall.user.as_ref(), |row, user| row.child(user.name.clone()).child(self.button("github-logout", "ログアウト / Sign out", ButtonStyle::secondary(self.wall.busy), cx, |this, _, cx| this.cloud_logout(cx)))))
                .when(!self.wall.configured, |row| row.child("クラウド未設定 / Cloud storage is not configured")))
            .child(div().flex_shrink_0().text_color(palette.muted).child(if self.wall.busy { "処理中 / Loading…".into() } else { self.status.clone() }))
            .child(div().id("my-wall-grid").flex_1().min_h_0().overflow_y_scroll()
                .child(div().flex().flex_wrap().gap_4()
                    .when(self.wall.user.is_some() && self.wall.works.is_empty() && !self.wall.busy, |grid| grid.child("まだ作品がありません。エディターから保存してください / Save your first work from the editor"))
                    .children(self.wall.works.iter().map(|work| {
                        let open = work.clone();
                        let delete = work.clone();
                        let drawing = Arc::new(Drawing::new(&Avatar::with_generation(
                            if work.settings.name.is_empty() { "blobatar" } else { &work.settings.name },
                            &work.settings.options, work.settings.generation()), &work.settings.options));
                        theme::panel(self.appearance).w(px(256.0)).p_4().gap_3()
                            .child(Blobatar::from_drawing(drawing).size(128.0))
                            .child(div().overflow_hidden().child(if work.settings.name.is_empty() { "blobatar".into() } else { work.settings.name.clone() }))
                            .child(self.button(format!("open-{}", work.id), "開く / Edit", ButtonStyle::secondary(self.wall.busy || self.busy), cx, move |this, _, cx| {
                                this.apply_cloud_settings(open.settings.clone(), cx);
                                this.wall.selected = Some(open.clone());
                                this.wall.delete_pending = None;
                                this.wall.open = false;
                                cx.notify();
                            }))
                            .child(self.button(format!("delete-{}", work.id), if self.wall.delete_pending.as_ref() == Some(&work.id) { "本当に削除 / Confirm delete" } else { "削除 / Delete" }, ButtonStyle::secondary(self.wall.busy), cx, move |this, _, cx| this.cloud_delete(&delete, cx)))
                    }))))
            .child(div().flex().gap_2().flex_shrink_0()
                .child(self.button("wall-prev", "前へ / Previous", ButtonStyle::secondary(self.wall.busy || self.wall.offset == 0 || self.wall.user.is_none()), cx, |this, _, cx| this.reload_cloud(this.wall.offset.saturating_sub(cloud::PAGE_SIZE), cx)))
                .child(self.button("wall-next", "次へ / Next", ButtonStyle::secondary(self.wall.busy || !self.wall.more || self.wall.user.is_none()), cx, |this, _, cx| this.reload_cloud(this.wall.offset + cloud::PAGE_SIZE, cx))))
    }
}
