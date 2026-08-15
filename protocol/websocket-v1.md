# Gytags WebSocket Protocol v1

Endpoint: `/ws`

## Client hello

```json
{
  "type": "hello",
  "protocol_version": 1,
  "token": "optional-token",
  "nicknames": ["setunicode", "Royalty72"]
}
```

Nicknames are normalized server-side. An empty `nicknames` array subscribes to the full snapshot.

## Server messages

```json
{
  "type": "hello_ack",
  "protocol_version": 1,
  "revision": 12,
  "source_status": "fresh",
  "source_age_seconds": 4
}
```

```json
{
  "type": "snapshot",
  "revision": 12,
  "source_status": "fresh",
  "source_age_seconds": 4,
  "members": [
    {
      "nickname": "setunicode",
      "status": "ok",
      "badges": ["academ"]
    }
  ]
}
```

Member statuses are `ok`, `ambiguous`, and `not_found`. Ambiguous nicknames never receive badges.

## Subscription updates

```json
{
  "type": "subscribe",
  "nicknames": ["setunicode", "Royalty72"]
}
```

The server responds with a new `snapshot`. When the stored snapshot changes, it sends an `update` containing the current revision and the members matching the current subscription.

## Internal collector ingest

Endpoint: `POST /internal/v1/ingest`.

When `GYTAGS_INGEST_TOKEN` is configured, send it in the `X-Gytags-Ingest-Token` header.

```json
{
  "type": "snapshot",
  "members": [
    {
      "discord_id": "606522922240180249",
      "nickname": "setunicode",
      "role_ids": ["1431742100940128347"]
    }
  ]
}
```
