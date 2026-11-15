# Stabled news feed format

Point `NEWS_FEED_URL` at an `https` URL that returns this JSON. A static file on any web host works.

```json
{
  "version": 1,
  "items": [
    {
      "id": "stabled-1-1-released",
      "title": "Stabled 1.1 is out",
      "summary": "One or two sentences shown on the card.",
      "category": "Update",
      "date": "2026-12-01",
      "image": "https://example.com/news/stabled-1-1.png",
      "url": "https://example.com/news/stabled-1-1",
      "body": "Optional full text for the reader view.\n\nBlank lines separate paragraphs."
    }
  ]
}
```

| Field | Required | Notes |
| --- | --- | --- |
| `id` | recommended | Stable identifier |
| `title` | yes | Up to 120 characters shown |
| `summary` | recommended | Up to 220 characters shown |
| `category` | no | Short label, default "Stabled" |
| `date` | no | `YYYY-MM-DD` or RFC 3339. Omit it rather than guessing — undated items are shown without a date |
| `image` | no | `https` only; hidden if it fails to load |
| `url` | no | `https` only; "Open article" opens it in the system browser |
| `body` | no | Plain text. If absent, the reader shows the summary |

Rules the launcher applies:

- At most 50 items are read. Items are sorted by date, newest first, together with Minecraft release notes.
- Results are cached in `%APPDATA%\Stabled\cache\news.json`. If a later fetch fails, the cache is shown with "Cached · <time ago>".
- Nothing from the feed is rendered as HTML.

Built-in guides (in `launcher/src-tauri/src/news.rs`) always follow the live items. They are product help, labelled "Guide", and never dated.
