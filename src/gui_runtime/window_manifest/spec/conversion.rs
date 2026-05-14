use super::WindowSpec;
use crate::gui_runtime::NativeRunOptions;

impl From<WindowSpec> for NativeRunOptions {
    fn from(spec: WindowSpec) -> Self {
        spec.into_native_options()
    }
}
