#[derive(Debug)]
pub struct ClusterSettings {}

impl ClusterSettings {
    pub fn dc_role_prefix() -> &'static str {
        "dc-"
    }

    pub fn default_data_center() -> &'static str {
        "default"
    }
}