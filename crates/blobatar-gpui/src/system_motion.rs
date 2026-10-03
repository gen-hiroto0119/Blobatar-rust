use gpui::{App, Global};

pub(crate) struct SystemMotion {
    pub reduced: bool,
    #[cfg(target_os = "macos")]
    _observer: mac::Observer,
    #[cfg(target_os = "macos")]
    _task: gpui::Task<()>,
}

impl Global for SystemMotion {}

pub(crate) fn current(cx: &mut App) -> bool {
    if !cx.has_global::<SystemMotion>() {
        install(cx);
    }
    cx.global::<SystemMotion>().reduced
}

#[cfg(not(target_os = "macos"))]
fn install(cx: &mut App) {
    cx.set_global(SystemMotion { reduced: false });
}

#[cfg(target_os = "macos")]
fn install(cx: &mut App) {
    use gpui::BorrowAppContext;
    let (sender, receiver) = async_channel::bounded(1);
    let observer = mac::Observer::new(sender);
    let reduced = mac::reduced_motion();
    let task = cx.spawn(async move |cx| {
        while receiver.recv().await.is_ok() {
            let result = cx.update(|cx| {
                let reduced = mac::reduced_motion();
                if cx.global::<SystemMotion>().reduced != reduced {
                    cx.update_global::<SystemMotion, _>(|preference, _| {
                        preference.reduced = reduced
                    });
                }
            });
            if result.is_err() {
                break;
            }
        }
    });
    cx.set_global(SystemMotion {
        reduced,
        _observer: observer,
        _task: task,
    });
}

#[cfg(target_os = "macos")]
mod mac {
    use block2::RcBlock;
    use objc2::{rc::Retained, runtime::ProtocolObject};
    use objc2_app_kit::{NSWorkspace, NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification};
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol};
    use std::ptr::NonNull;

    pub fn reduced_motion() -> bool {
        NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
    }

    pub struct Observer {
        center: Retained<NSNotificationCenter>,
        token: Retained<ProtocolObject<dyn NSObjectProtocol>>,
    }

    impl Observer {
        pub fn new(sender: async_channel::Sender<()>) -> Self {
            let center = NSWorkspace::sharedWorkspace().notificationCenter();
            let callback = RcBlock::new(move |_: NonNull<NSNotification>| {
                let _ = sender.try_send(());
            });
            // The block captures only a Send channel. UI state is read on GPUI's main thread.
            let token = unsafe {
                center.addObserverForName_object_queue_usingBlock(
                    Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
                    None,
                    None,
                    &callback,
                )
            };
            Self { center, token }
        }
    }

    impl Drop for Observer {
        fn drop(&mut self) {
            // The token is retained from this center's addObserver call.
            unsafe {
                self.center.removeObserver((*self.token).as_ref());
            }
        }
    }
}
