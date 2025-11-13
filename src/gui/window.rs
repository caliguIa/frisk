use log::debug;
use objc2::define_class;
use objc2_app_kit::NSPanel;

define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "KickoffCustomWindow"]
    pub struct CustomWindow;

    impl CustomWindow {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            debug!("canBecomeKeyWindow called");
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            debug!("canBecomeMainWindow called");
            true
        }
    }
);
