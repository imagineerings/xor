# Channel Message Search Performance Notes

## Query Plan Check

Run this against a staging-sized Postgres database after the search migration has
been applied:

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT cm.id
FROM channel_messages cm
JOIN channels c ON c.id = cm.channel_id
JOIN channel_members member ON member.channel_id = c.root_channel_id
WHERE member.user_id = $1
  AND member.accepted = TRUE
  AND member.role != 'banned'
  AND (member.role IN ('admin', 'member') OR c.visibility = 'public')
  AND cm.deleted_at IS NULL
  AND cm.search_vector @@ to_tsquery('english', $2)
ORDER BY ts_rank(cm.search_vector, to_tsquery('english', $2)) DESC, cm.id DESC
LIMIT 21;
```

Expected shape:

- Postgres should use `idx_channel_messages_search` through a bitmap index scan
  or equivalent GIN-backed plan for the `search_vector @@ ...` predicate.
- The access-control joins should filter the candidate set after the full-text
  index narrows matching messages.
- For the default UI page size, the query asks for `limit + 1` rows so the
  server can return an accurate `done` flag without a second count query.

## Current Limits

- UI default page size: 20 results.
- Server maximum page size: 100 results.
- Query text is capped at 200 characters before building a `tsquery`.
- Postgres search uses prefix terms (`term:*`) for partial-word matching.
- SQLite test search intentionally uses a `LOWER(body) LIKE` fallback and is not
  representative of production performance.

## Tuning Follow-Ups

- Add `pg_trgm` only if prefix full-text matching is not sufficient for users.
  It would add write overhead and another index, so it should be justified by
  real search examples.
- Revisit ranking if large channels show expensive `ts_rank` sorts. The first
  fallback should be reducing the maximum page size or adding a recency tie
  breaker before adding more indexes.
