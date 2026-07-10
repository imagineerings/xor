use crate::{Client, Subscription};
use anyhow::Result;
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

impl GroupStore {
    pub fn new(client: Arc<Client>, cx: &mut Context<Self>) -> Self {
        let subscriptions =
            vec![client.add_message_handler(cx.weak_entity(), Self::handle_update_groups)];
        let load_groups = cx.spawn({
            let client = client.clone();
            async move |this, cx| {
                let groups = client.request(proto::GetGroups {}).await?.groups;
                this.update(cx, |this, cx| this.replace_groups(groups, cx))?;
                Ok(())
            }
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
