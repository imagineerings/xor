use crate::{Client, Subscription};
use anyhow::{Context as _, Result};
use collections::HashMap;
use gpui::{AsyncApp, Context, Entity, EventEmitter, SharedString, Task};
use rpc::{TypedEnvelope, proto};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub id: u64,
    pub name: SharedString,
    pub display_name: SharedString,
    pub admin_id: u64,
    pub member_ids: Vec<u64>,
}

impl From<proto::UserGroup> for Group {
    fn from(group: proto::UserGroup) -> Self {
        Self {
            id: group.id,
            name: group.name.into(),
            display_name: group.display_name.into(),
            admin_id: group.admin_id,
            member_ids: group.member_ids,
        }
    }
}

#[derive(Clone, Debug)]
pub enum GroupStoreEvent {
    GroupsUpdated,
    GroupMembershipChanged { group_id: u64 },
}

pub struct GroupStore {
    groups: HashMap<u64, Arc<Group>>,
    by_name: HashMap<SharedString, Arc<Group>>,
    user_groups: HashMap<u64, Vec<Arc<Group>>>,
    _subscriptions: Vec<Subscription>,
    _load_groups: Task<Result<()>>,
}

impl EventEmitter<GroupStoreEvent> for GroupStore {}

impl Client {
    pub async fn create_group(
        &self,
        name: String,
        display_name: String,
        member_ids: Vec<u64>,
    ) -> Result<Group> {
        let response = self
            .request(proto::CreateGroup {
                name,
                display_name,
                member_ids,
            })
            .await?;
        response
            .group
            .map(Into::into)
            .context("missing created group")
    }

    pub async fn update_group(
        &self,
        group_id: u64,
        name: Option<String>,
        display_name: Option<String>,
    ) -> Result<Group> {
        let response = self
            .request(proto::UpdateGroup {
                group_id,
                name,
                display_name,
            })
            .await?;
        response
            .group
            .map(Into::into)
            .context("missing updated group")
    }

    pub async fn delete_group(&self, group_id: u64) -> Result<()> {
        self.request(proto::DeleteGroup { group_id })
            .await
            .map(|_: proto::DeleteGroupResponse| ())
    }

    pub async fn update_group_members(
        &self,
        group_id: u64,
        add_user_ids: Vec<u64>,
        remove_user_ids: Vec<u64>,
    ) -> Result<Group> {
        let response = self
            .request(proto::UpdateGroupMembers {
                group_id,
                add_user_ids,
                remove_user_ids,
            })
            .await?;
        response
            .group
            .map(Into::into)
            .context("missing updated group")
    }

    pub async fn leave_group(&self, group_id: u64) -> Result<()> {
        self.request(proto::LeaveGroup { group_id })
            .await
            .map(|_: proto::LeaveGroupResponse| ())
    }
}

impl GroupStore {
    pub fn new(client: Arc<Client>, cx: &mut Context<Self>) -> Self {
        let subscriptions =
            vec![client.add_message_handler(cx.weak_entity(), Self::handle_update_groups)];
        let load_groups = cx.spawn(async move |this, cx| {
            let groups = client.request(proto::GetGroups {}).await?.groups;
            this.update(cx, |this, cx| this.replace_groups(groups, cx))?;
            Ok(())
        });
        Self {
            groups: HashMap::default(),
            by_name: HashMap::default(),
            user_groups: HashMap::default(),
            _subscriptions: subscriptions,
            _load_groups: load_groups,
        }
    }

    pub fn search_groups(&self, query: &str) -> Vec<Arc<Group>> {
        let query = query.to_lowercase();
        let mut groups = self
            .groups
            .values()
            .filter(|group| {
                group.name.to_lowercase().starts_with(&query)
                    || group.display_name.to_lowercase().starts_with(&query)
            })
            .cloned()
            .collect::<Vec<_>>();
        groups.sort_by(|left, right| left.name.cmp(&right.name));
        groups
    }

    pub fn is_member(&self, group_id: u64, user_id: u64) -> bool {
        self.groups
            .get(&group_id)
            .is_some_and(|group| group.member_ids.contains(&user_id))
    }

    pub fn all_groups(&self) -> Vec<Arc<Group>> {
        let mut groups = self.groups.values().cloned().collect::<Vec<_>>();
        groups.sort_by(|left, right| left.name.cmp(&right.name));
        groups
    }

    pub fn group(&self, group_id: u64) -> Option<Arc<Group>> {
        self.groups.get(&group_id).cloned()
    }

    pub fn groups_for_user(&self, user_id: u64) -> &[Arc<Group>] {
        self.user_groups
            .get(&user_id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    async fn handle_update_groups(
        this: Entity<Self>,
        envelope: TypedEnvelope<proto::UpdateGroups>,
        mut cx: AsyncApp,
    ) -> Result<()> {
        this.update(&mut cx, |this, cx| this.apply_update(envelope.payload, cx));
        Ok(())
    }

    fn replace_groups(&mut self, groups: Vec<proto::UserGroup>, cx: &mut Context<Self>) {
        self.groups.clear();
        for group in groups {
            self.groups.insert(group.id, Arc::new(group.into()));
        }
        self.rebuild_indexes();
        cx.emit(GroupStoreEvent::GroupsUpdated);
        cx.notify();
    }

    fn apply_update(&mut self, update: proto::UpdateGroups, cx: &mut Context<Self>) {
        for group_id in update.delete_group_ids {
            self.groups.remove(&group_id);
        }
        for group in update.groups {
            let group_id = group.id;
            self.groups.insert(group_id, Arc::new(group.into()));
            cx.emit(GroupStoreEvent::GroupMembershipChanged { group_id });
        }
        self.rebuild_indexes();
        cx.emit(GroupStoreEvent::GroupsUpdated);
        cx.notify();
    }

    fn rebuild_indexes(&mut self) {
        self.by_name.clear();
        self.user_groups.clear();
        for group in self.groups.values() {
            self.by_name.insert(group.name.clone(), group.clone());
            for user_id in &group.member_ids {
                self.user_groups
                    .entry(*user_id)
                    .or_default()
                    .push(group.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn search_groups_matches_name_and_display_name_prefixes_case_insensitively() {
        let engineering = Arc::new(Group {
            id: 1,
            name: "eng-team".into(),
            display_name: "Engineering Team".into(),
            admin_id: 1,
            member_ids: Vec::new(),
        });
        let design = Arc::new(Group {
            id: 2,
            name: "design".into(),
            display_name: "Product Design".into(),
            admin_id: 1,
            member_ids: Vec::new(),
        });
        let store = GroupStore {
            groups: [
                (engineering.id, engineering.clone()),
                (design.id, design.clone()),
            ]
            .into_iter()
            .collect(),
            by_name: HashMap::default(),
            user_groups: HashMap::default(),
            _subscriptions: Vec::new(),
            _load_groups: Task::ready(Ok(())),
        };

        assert_eq!(store.search_groups("ENG"), vec![engineering]);
        assert_eq!(store.search_groups("product"), vec![design]);
        assert!(store.search_groups("marketing").is_empty());
    }

    proptest! {
        #[test]
        fn search_groups_is_prefix_closed(name in "[a-z]{1,12}") {
            let group = Arc::new(Group {
                id: 1,
                name: name.clone().into(),
                display_name: "Team".into(),
                admin_id: 1,
                member_ids: Vec::new(),
            });
            let store = GroupStore {
                groups: [(group.id, group.clone())].into_iter().collect(),
                by_name: HashMap::default(),
                user_groups: HashMap::default(),
                _subscriptions: Vec::new(),
                _load_groups: Task::ready(Ok(())),
            };
            prop_assert!(store.search_groups(&name[..1]).contains(&group));
        }
    }
}
