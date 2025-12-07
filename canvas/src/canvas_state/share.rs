use std::sync::*;

///
/// Canvas brush that can be modified or shared
///
#[derive(Clone, Debug)]
pub enum CanvasShared<T> {
    Modified(T),
    Shared(Arc<T>)
}
