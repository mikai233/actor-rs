#[derive(Debug, Default)]
pub(super) enum CoordinatorState {
    #[default]
    WaitingForStateInitialized,
    Active,
}