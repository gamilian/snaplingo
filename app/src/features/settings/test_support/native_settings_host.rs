//! Temporary shell boundary: the retained navigation supplies a rectangle;
//! AppKit and Rust services own each migrated preferences page.
use std::cell::RefCell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSView, NSWindow};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSObjectProtocol, NSPoint, NSRect, NSSize,
};
use serde::Deserialize;

use super::{GeneralPage, SharedFeaturePage};
use snaplingo_core::settings::GeneralSection;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsViewport {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

impl SettingsViewport {
    fn validate(self) -> Result<Self, String> {
        if ![self.x, self.y, self.width, self.height, self.scale]
            .into_iter()
            .all(f64::is_finite)
            || self.x < 0.0
            || self.y < 0.0
            || self.width <= 0.0
            || self.height <= 0.0
            || !(0.5..=3.0).contains(&self.scale)
        {
            return Err("Invalid native Settings page bounds".into());
        }
        Ok(self)
    }
}

pub(crate) enum NativePage {
    General(Rc<GeneralPage>),
    Feature(Rc<SharedFeaturePage>),
}

impl NativePage {
    fn view(&self) -> &NSView {
        match self {
            Self::General(page) => page.view(),
            Self::Feature(page) => page.view(),
        }
    }
    fn refresh(&self) -> Result<(), String> {
        match self {
            Self::General(page) => page.refresh(),
            Self::Feature(page) => page.refresh(),
        }
    }
    fn set_layout(&self, shell: f64, viewport: f64, size: NSSize) {
        match self {
            Self::General(page) => {
                page.view().set_layout_widths(shell, viewport);
                page.view().setFrameSize(size);
            }
            Self::Feature(page) => page.view().set_layout(shell, viewport, size),
        }
    }
    fn jump(&self, section: &str) -> Result<(), String> {
        match self {
            Self::General(page) => {
                page.view().jump(parse_section(section)?);
                Ok(())
            }
            Self::Feature(page) => page.jump(section),
        }
    }
    fn suspend(&self) {
        if let Self::Feature(page) = self {
            page.suspend();
        }
    }
    fn dispose(&self) {
        match self {
            Self::General(page) => page.dispose(),
            Self::Feature(page) => page.dispose(),
        }
    }
}

impl From<Rc<GeneralPage>> for NativePage {
    fn from(page: Rc<GeneralPage>) -> Self {
        Self::General(page)
    }
}
impl From<Rc<SharedFeaturePage>> for NativePage {
    fn from(page: Rc<SharedFeaturePage>) -> Self {
        Self::Feature(page)
    }
}

struct HostIvars {
    page: NativePage,
}
define_class!(
    #[unsafe(super(NSView))]
    #[name = "SnapLingoSettingsPageHost"]
    #[thread_kind = MainThreadOnly]
    #[ivars = HostIvars]
    struct Host;
    unsafe impl NSObjectProtocol for Host {}
    impl Host {
        #[unsafe(method(isFlipped))]
        fn flipped(&self) -> bool { true }
        #[unsafe(method(settingsWindowFocused:))]
        fn focused(&self, _: &NSNotification) {
            if let Err(error) = self.ivars().page.refresh() {
                log::warn!("Failed to refresh native preferences on focus: {error}");
            }
        }
    }
);

impl Drop for Host {
    fn drop(&mut self) {
        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver(self);
        }
    }
}

struct Mounted {
    token: String,
    host: Retained<Host>,
}
thread_local! { static MOUNTED: RefCell<Option<Mounted>> = const { RefCell::new(None) }; }

pub(crate) fn present(
    parent: &NSView,
    token: String,
    viewport: SettingsViewport,
    section: Option<&str>,
    create: impl FnOnce() -> Result<NativePage, String>,
) -> Result<(), String> {
    let viewport = viewport.validate()?;
    let current = MOUNTED.with_borrow(|slot| {
        slot.as_ref().and_then(|mounted| {
            (mounted.token == token
                && unsafe { mounted.host.superview() }.is_some_and(|root| &*root == parent))
            .then(|| mounted.host.clone())
        })
    });
    let host = if let Some(host) = current {
        host
    } else {
        // Construct before detaching the previous page: a service/read failure
        // must leave the current preferences usable.
        let page = create()?;
        dispose();
        let host = Host::alloc(parent.mtm()).set_ivars(HostIvars { page });
        let host: Retained<Host> = unsafe { msg_send![super(host), initWithFrame: NSRect::ZERO] };
        host.addSubview(host.ivars().page.view());
        parent.addSubview(&host);
        if let Some(window) = parent.window() {
            unsafe {
                NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                    &host,
                    sel!(settingsWindowFocused:),
                    Some(objc2_app_kit::NSWindowDidBecomeKeyNotification),
                    Some(&window),
                );
            }
        }
        MOUNTED.with_borrow_mut(|slot| {
            *slot = Some(Mounted {
                token,
                host: host.clone(),
            })
        });
        host
    };
    let frame = NSRect::new(
        NSPoint::new(
            viewport.x,
            if parent.isFlipped() {
                viewport.y
            } else {
                parent.bounds().size.height - viewport.y - viewport.height
            },
        ),
        NSSize::new(viewport.width, viewport.height),
    );
    let size = NSSize::new(
        viewport.width / viewport.scale,
        viewport.height / viewport.scale,
    );
    host.setFrame(frame);
    host.setBoundsSize(size);
    host.ivars().page.set_layout(
        parent.bounds().size.width / viewport.scale,
        parent.bounds().size.width,
        size,
    );
    if let Some(section) = section {
        host.ivars().page.jump(section)?;
    }
    host.setNeedsDisplay(true);
    Ok(())
}

pub(crate) fn dismiss(token: &str) {
    if MOUNTED.with_borrow(|slot| slot.as_ref().is_some_and(|mounted| mounted.token == token)) {
        dispose();
    }
}

pub(crate) fn prepare_hide(window: &NSWindow) {
    super::notices::close_for(window);
    let host = MOUNTED.with_borrow(|slot| slot.as_ref().map(|mounted| mounted.host.clone()));
    if let Some(host) = host {
        host.ivars().page.suspend();
    }
    // Commits a text field just as the retained Settings page does on blur.
    window.makeFirstResponder(None);
}

pub(crate) fn dispose() {
    let mounted = MOUNTED.with_borrow_mut(Option::take);
    if let Some(mounted) = mounted {
        if let Some(window) = mounted.host.window() {
            prepare_hide(&window);
        }
        mounted.host.ivars().page.dispose();
        mounted.host.removeFromSuperview();
    }
}

fn parse_section(id: &str) -> Result<GeneralSection, String> {
    GeneralSection::ALL
        .into_iter()
        .find(|section| section.id() == id)
        .ok_or_else(|| "Unknown General preferences section".into())
}

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn test_mounted_view() -> Option<Retained<NSView>> {
    MOUNTED.with_borrow(|slot| {
        slot.as_ref()
            .map(|mounted| mounted.host.clone().into_super())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_bounds_and_routes_do_not_reach_appkit() {
        let valid = SettingsViewport {
            x: 202.0,
            y: 0.0,
            width: 698.0,
            height: 650.0,
            scale: 1.0,
        };
        assert!(valid.validate().is_ok());
        for viewport in [
            SettingsViewport {
                width: f64::NAN,
                ..valid
            },
            SettingsViewport {
                scale: 0.0,
                ..valid
            },
            SettingsViewport {
                height: -1.0,
                ..valid
            },
        ] {
            assert!(viewport.validate().is_err());
        }
        assert_eq!(parse_section("about").unwrap(), GeneralSection::About);
        assert!(parse_section("invalid").is_err());
    }
}
