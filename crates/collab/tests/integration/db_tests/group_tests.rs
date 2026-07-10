use super::new_test_user;
use crate::test_both_dbs;
use collab::db::{Database, queries::groups::MAX_GROUP_MEMBERS};
use std::sync::Arc;

test_both_dbs!(
    test_group_queries,
    test_group_queries_postgres,
    test_group_queries_sqlite
);

async fn test_group_queries(db: &Arc<Database>) {
    let admin_id = new_test_user(db).await;
    let member_id = new_test_user(db).await;
    let other_member_id = new_test_user(db).await;

    assert!(
        db.create_group("bad group", "Bad group", admin_id, &[])
            .await
            .is_err()
    );

    let group = db
        .create_group("eng-team", "Engineering", admin_id, &[member_id])
        .await
        .unwrap();
    assert_member_ids(group.member_ids.clone(), vec![admin_id, member_id]);
    assert!(!db.is_group_name_available("eng-team").await.unwrap());
    assert!(db.is_group_name_available("design-team").await.unwrap());
    assert!(
        db.create_group("eng-team", "Duplicate", admin_id, &[])
            .await
            .is_err()
    );

    let updated = db
        .update_group_members(group.group.id, &[member_id, other_member_id], &[])
        .await
        .unwrap();
    assert_member_ids(
        updated.member_ids,
        vec![admin_id, member_id, other_member_id],
    );

    let updated = db
        .update_group_members(group.group.id, &[other_member_id], &[member_id, member_id])
        .await
        .unwrap();
    assert_member_ids(updated.member_ids, vec![admin_id, other_member_id]);

    db.leave_group(group.group.id, other_member_id)
        .await
        .unwrap();
    let group_after_leave = db.get_group(group.group.id).await.unwrap().unwrap();
    assert_member_ids(group_after_leave.member_ids, vec![admin_id]);
    assert_eq!(
        db.get_groups_for_user(other_member_id).await.unwrap().len(),
        0
    );

    db.leave_group(group.group.id, admin_id).await.unwrap();
    let empty_group = db.get_group(group.group.id).await.unwrap().unwrap();
    assert!(empty_group.member_ids.is_empty());
    assert_eq!(db.group_member_count(group.group.id).await.unwrap(), 0);

    let max_members = (0..MAX_GROUP_MEMBERS)
        .map(|_| new_test_user(db))
        .collect::<Vec<_>>();
    let mut max_members = futures::future::join_all(max_members).await;
    max_members.push(admin_id);
    assert!(
        db.create_group("too-many", "Too many", admin_id, &max_members)
            .await
            .is_err()
    );
}

fn assert_member_ids(mut actual: Vec<collab::db::UserId>, mut expected: Vec<collab::db::UserId>) {
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}
