use super::*;
use anyhow::{Context as _, anyhow};

pub const MAX_GROUP_MEMBERS: usize = 100;

#[derive(Clone, Debug)]
pub struct GroupWithMembers {
    pub group: user_group::Model,
    pub member_ids: Vec<UserId>,
}

impl GroupWithMembers {
    pub fn to_proto(&self) -> proto::UserGroup {
        proto::UserGroup {
            id: self.group.id.to_proto(),
            name: self.group.name.clone(),
            display_name: self.group.display_name.clone(),
            admin_id: self.group.admin_id.to_proto(),
            member_ids: self.member_ids.iter().map(|id| id.to_proto()).collect(),
        }
    }
}

impl Database {
    pub async fn create_group(
        &self,
        name: &str,
        display_name: &str,
        admin_id: UserId,
        member_ids: &[UserId],
    ) -> Result<GroupWithMembers> {
        validate_group_name(name)?;
        if display_name.trim().is_empty() {
            return Err(anyhow!("group display name cannot be empty").into());
        }
        let member_ids = group_member_ids(member_ids, admin_id)?;
        let name = name.to_string();
        let display_name = display_name.to_string();
        self.transaction(move |tx| {
            let name = name.clone();
            let display_name = display_name.clone();
            let member_ids = member_ids.clone();
            async move {
                if user_group::Entity::find()
                    .filter(user_group::Column::Name.eq(name.clone()))
                    .one(&*tx)
                    .await?
                    .is_some()
                {
                    return Err(anyhow!("group name already exists").into());
                }
                let group = user_group::ActiveModel {
                    id: ActiveValue::NotSet,
                    name: ActiveValue::Set(name.clone()),
                    display_name: ActiveValue::Set(display_name.clone()),
                    admin_id: ActiveValue::Set(admin_id),
                    created_at: ActiveValue::NotSet,
                    updated_at: ActiveValue::NotSet,
                }
                .insert(&*tx)
                .await?;
                insert_group_members(group.id, &member_ids, &tx).await?;
                Ok(GroupWithMembers { group, member_ids })
            }
        })
        .await
    }

    pub async fn update_group(
        &self,
        group_id: GroupId,
        name: Option<&str>,
        display_name: Option<&str>,
    ) -> Result<GroupWithMembers> {
        if let Some(name) = name {
            validate_group_name(name)?;
        }
        if display_name.is_some_and(|display_name| display_name.trim().is_empty()) {
            return Err(anyhow!("group display name cannot be empty").into());
        }
        let name = name.map(str::to_string);
        let display_name = display_name.map(str::to_string);
        self.transaction(move |tx| {
            let name = name.clone();
            let display_name = display_name.clone();
            async move {
                let group = get_group_model(group_id, &tx).await?;
                if let Some(name) = &name
                    && name != &group.name
                    && user_group::Entity::find()
                        .filter(user_group::Column::Name.eq(name))
                        .one(&*tx)
                        .await?
                        .is_some()
                {
                    return Err(anyhow!("group name already exists").into());
                }
                let group = user_group::Entity::update(user_group::ActiveModel {
                    id: ActiveValue::Unchanged(group.id),
                    name: ActiveValue::Set(name.as_deref().unwrap_or(&group.name).to_string()),
                    display_name: ActiveValue::Set(
                        display_name
                            .as_deref()
                            .unwrap_or(&group.display_name)
                            .to_string(),
                    ),
                    admin_id: ActiveValue::Unchanged(group.admin_id),
                    created_at: ActiveValue::Unchanged(group.created_at),
                    updated_at: ActiveValue::Set(current_time()),
                })
                .exec(&*tx)
                .await?;
                let member_ids = group_member_ids_for_group(group.id, &tx).await?;
                Ok(GroupWithMembers { group, member_ids })
            }
        })
        .await
    }

    pub async fn delete_group(&self, group_id: GroupId) -> Result<()> {
        self.transaction(move |tx| async move {
            let result = user_group::Entity::delete_by_id(group_id)
                .exec(&*tx)
                .await?;
            if result.rows_affected == 0 {
                return Err(anyhow!("group not found").into());
            }
            Ok(())
        })
        .await
    }

    pub async fn get_groups(&self) -> Result<Vec<GroupWithMembers>> {
        self.transaction(|tx| async move {
            let groups = user_group::Entity::find()
                .order_by_asc(user_group::Column::Name)
                .all(&*tx)
                .await?;
            groups_with_members(groups, &tx).await
        })
        .await
    }

    pub async fn get_group(&self, group_id: GroupId) -> Result<Option<GroupWithMembers>> {
        self.transaction(move |tx| async move {
            let Some(group) = user_group::Entity::find_by_id(group_id).one(&*tx).await? else {
                return Ok(None);
            };
            Ok(Some(GroupWithMembers {
                member_ids: group_member_ids_for_group(group.id, &tx).await?,
                group,
            }))
        })
        .await
    }

    pub async fn update_group_members(
        &self,
        group_id: GroupId,
        add_ids: &[UserId],
        remove_ids: &[UserId],
    ) -> Result<GroupWithMembers> {
        let add_ids = deduplicate_user_ids(add_ids);
        let remove_ids = deduplicate_user_ids(remove_ids);
        self.transaction(move |tx| {
            let add_ids = add_ids.clone();
            let remove_ids = remove_ids.clone();
            async move {
                let group = get_group_model(group_id, &tx).await?;
                let mut member_ids = group_member_ids_for_group(group_id, &tx).await?;
                member_ids.retain(|id| !remove_ids.contains(id));
                let additions = add_ids
                    .iter()
                    .copied()
                    .filter(|id| !member_ids.contains(id))
                    .collect::<Vec<_>>();
                member_ids.extend(additions);
                if member_ids.len() > MAX_GROUP_MEMBERS {
                    return Err(anyhow!(
                        "group exceeds maximum member count of {MAX_GROUP_MEMBERS}"
                    )
                    .into());
                }
                if !remove_ids.is_empty() {
                    user_group_member::Entity::delete_many()
                        .filter(user_group_member::Column::GroupId.eq(group_id))
                        .filter(user_group_member::Column::UserId.is_in(remove_ids))
                        .exec(&*tx)
                        .await?;
                }
                let existing = group_member_ids_for_group(group_id, &tx).await?;
                let new_ids = member_ids
                    .iter()
                    .copied()
                    .filter(|id| !existing.contains(id))
                    .collect::<Vec<_>>();
                insert_group_members(group_id, &new_ids, &tx).await?;
                Ok(GroupWithMembers { group, member_ids })
            }
        })
        .await
    }

    pub async fn leave_group(&self, group_id: GroupId, user_id: UserId) -> Result<()> {
        self.transaction(move |tx| async move {
            let result = user_group_member::Entity::delete_many()
                .filter(user_group_member::Column::GroupId.eq(group_id))
                .filter(user_group_member::Column::UserId.eq(user_id))
                .exec(&*tx)
                .await?;
            if result.rows_affected == 0 {
                return Err(anyhow!("group membership not found").into());
            }
            Ok(())
        })
        .await
    }

    pub async fn get_group_member_ids(&self, group_id: GroupId) -> Result<Vec<UserId>> {
        self.transaction(move |tx| async move {
            get_group_model(group_id, &tx).await?;
            group_member_ids_for_group(group_id, &tx).await
        })
        .await
    }

    pub async fn get_groups_for_user(&self, user_id: UserId) -> Result<Vec<GroupWithMembers>> {
        self.transaction(move |tx| async move {
            let group_ids = user_group_member::Entity::find()
                .filter(user_group_member::Column::UserId.eq(user_id))
                .all(&*tx)
                .await?
                .into_iter()
                .map(|member| member.group_id)
                .collect::<Vec<_>>();
            let groups = user_group::Entity::find()
                .filter(user_group::Column::Id.is_in(group_ids))
                .order_by_asc(user_group::Column::Name)
                .all(&*tx)
                .await?;
            groups_with_members(groups, &tx).await
        })
        .await
    }

    pub async fn is_group_name_available(&self, name: &str) -> Result<bool> {
        self.transaction(move |tx| async move {
            Ok(user_group::Entity::find()
                .filter(user_group::Column::Name.eq(name))
                .one(&*tx)
                .await?
                .is_none())
        })
        .await
    }

    pub async fn group_member_count(&self, group_id: GroupId) -> Result<usize> {
        Ok(self.get_group_member_ids(group_id).await?.len())
    }
}

fn validate_group_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(anyhow!("group name must contain only letters, numbers, and hyphens").into());
    }
    Ok(())
}

fn group_member_ids(member_ids: &[UserId], admin_id: UserId) -> Result<Vec<UserId>> {
    let mut member_ids = deduplicate_user_ids(member_ids);
    if !member_ids.contains(&admin_id) {
        member_ids.push(admin_id);
    }
    if member_ids.len() > MAX_GROUP_MEMBERS {
        return Err(anyhow!("group exceeds maximum member count of {MAX_GROUP_MEMBERS}").into());
    }
    Ok(member_ids)
}

fn deduplicate_user_ids(member_ids: &[UserId]) -> Vec<UserId> {
    let mut member_ids = member_ids.to_vec();
    member_ids.sort_unstable();
    member_ids.dedup();
    member_ids
}

async fn get_group_model(group_id: GroupId, tx: &DatabaseTransaction) -> Result<user_group::Model> {
    Ok(user_group::Entity::find_by_id(group_id)
        .one(tx)
        .await?
        .context("group not found")?)
}

async fn group_member_ids_for_group(
    group_id: GroupId,
    tx: &DatabaseTransaction,
) -> Result<Vec<UserId>> {
    Ok(user_group_member::Entity::find()
        .filter(user_group_member::Column::GroupId.eq(group_id))
        .order_by_asc(user_group_member::Column::Id)
        .all(tx)
        .await?
        .into_iter()
        .map(|member| member.user_id)
        .collect())
}

async fn groups_with_members(
    groups: Vec<user_group::Model>,
    tx: &DatabaseTransaction,
) -> Result<Vec<GroupWithMembers>> {
    let mut groups_with_members = Vec::with_capacity(groups.len());
    for group in groups {
        groups_with_members.push(GroupWithMembers {
            member_ids: group_member_ids_for_group(group.id, tx).await?,
            group,
        });
    }
    Ok(groups_with_members)
}

async fn insert_group_members(
    group_id: GroupId,
    member_ids: &[UserId],
    tx: &DatabaseTransaction,
) -> Result<()> {
    if member_ids.is_empty() {
        return Ok(());
    }
    user_group_member::Entity::insert_many(member_ids.iter().copied().map(|user_id| {
        user_group_member::ActiveModel {
            id: ActiveValue::NotSet,
            group_id: ActiveValue::Set(group_id),
            user_id: ActiveValue::Set(user_id),
            created_at: ActiveValue::NotSet,
        }
    }))
    .exec_without_returning(tx)
    .await?;
    Ok(())
}

fn current_time() -> time::PrimitiveDateTime {
    let now = time::OffsetDateTime::now_utc();
    time::PrimitiveDateTime::new(now.date(), now.time())
}
