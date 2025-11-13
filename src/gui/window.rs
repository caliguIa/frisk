use log::debug;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, sel, sel_impl};

pub struct CustomWindow;

impl CustomWindow {
    const NAME: &'static str = "KickoffCustomWindow";

    fn define_class() -> &'static Class {
        let mut decl = ClassDecl::new(Self::NAME, class!(NSPanel))
            .unwrap_or_else(|| panic!("Unable to register {} class", Self::NAME));

        unsafe {
            decl.add_method(
                sel!(canBecomeKeyWindow),
                Self::can_become_key_window as extern "C" fn(&Object, Sel) -> bool,
            );

            decl.add_method(
                sel!(canBecomeMainWindow),
                Self::can_become_main_window as extern "C" fn(&Object, Sel) -> bool,
            );
        }

        decl.register()
    }

    extern "C" fn can_become_key_window(_this: &Object, _sel: Sel) -> bool {
        debug!("canBecomeKeyWindow called");
        true
    }

    extern "C" fn can_become_main_window(_this: &Object, _sel: Sel) -> bool {
        debug!("canBecomeMainWindow called");
        true
    }

    pub fn class() -> &'static Class {
        Class::get(Self::NAME).unwrap_or_else(Self::define_class)
    }
}
