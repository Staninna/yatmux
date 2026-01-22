/// Direction of a split between panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDir {
    /// Split left/right.
    Vertical,
    /// Split top/bottom.
    Horizontal,
}
