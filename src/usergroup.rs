use crate::{DaemonizeError, Group, User};

#[derive(Default)]
pub struct UserGroup {
    pub user: Option<User>,
    pub group: Option<Group>,
}

impl UserGroup {
    /// Returns current UserGroup
    #[cfg(target_family = "unix")]
    pub fn get() -> Result<UserGroup, DaemonizeError> {
        use users::{get_current_groupname, get_current_username};
        let user = get_current_username()
            .ok_or_else(|| DaemonizeError::UserNotFound)?
            .to_str()
            .ok_or_else(|| DaemonizeError::EncodeUser)?
            .to_string();
        let group = get_current_groupname()
            .ok_or_else(|| DaemonizeError::GroupNotFound)?
            .to_str()
            .ok_or_else(|| DaemonizeError::EncodeGroup)?
            .to_string();
        Ok(UserGroup {
            user: Some(user.into()),
            group: Some(group.into()),
        })
    }
    #[cfg(not(target_family = "unix"))]
    pub fn get() -> Result<UserGroup, DaemonizeError> {
        Ok(UserGroup::default())
    }
}
