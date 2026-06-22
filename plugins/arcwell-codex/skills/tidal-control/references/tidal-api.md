# TIDAL API Notes

The script uses an existing authenticated TIDAL session. It never stores tokens
in the repo or prints token values.

## Authentication

Use either:

- `TIDAL_ACCESS_TOKEN` plus `TIDAL_CLIENT_ID`
- macOS TIDAL desktop IndexedDB data under
  `~/Library/Application Support/TIDAL/IndexedDB/https_desktop.tidal.com_0.indexeddb.leveldb`

The access token JWT supplies `uid` and `cc` when present.

## Endpoints

Read/search:

- `GET https://api.tidal.com/v1/search/tracks?query=...&countryCode=...&limit=...`
- `GET https://api.tidal.com/v1/users/{userId}/playlists?countryCode=...&limit=...`
- `GET https://api.tidal.com/v1/playlists/{playlistId}/tracks?countryCode=...&limit=...`

Playlist writes:

- `POST https://openapi.tidal.com/v2/playlists`
- `PATCH https://openapi.tidal.com/v2/playlists/{playlistId}`
- `POST https://openapi.tidal.com/v2/playlists/{playlistId}/relationships/items`

Collection/favorite writes:

- `POST https://openapi.tidal.com/v2/userCollections/{userId}/relationships/tracks`
- `POST https://openapi.tidal.com/v2/userCollections/{userId}/relationships/playlists`

## JSON:API Payloads

Create playlist:

```json
{
  "data": {
    "type": "playlists",
    "attributes": {
      "name": "grunge",
      "description": "optional",
      "accessType": "UNLISTED"
    }
  }
}
```

Add tracks to playlist:

```json
{
  "data": [
    { "id": "23688046", "type": "tracks" }
  ]
}
```

Favorite a playlist:

```json
{
  "data": [
    { "id": "8fb008d2-8bca-4de9-b033-2bab4fde8e15", "type": "playlists" }
  ]
}
```

## Matching Discipline

Prefer exact artist and title matches. Penalize karaoke, tribute, cover, live,
demo, acoustic, remix, and instrumental results unless the user asks for them.
If a match is low-confidence, show the candidate and ask or use `search-tracks`
to inspect alternatives before writing.
