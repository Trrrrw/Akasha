```
GET /v1/games
GET /v1/games/{game}
```

```
GET /v1/news/items
GET /v1/news/items/{item_id}
GET /v1/news/items/{item_id}/content
GET /v1/news/items/{item_id}/related

GET /v1/news/games
GET /v1/news/games/{game_id}
GET /v1/news/games/{game_id}/items
GET /v1/news/games/{game_id}/categories

GET /v1/news/categories
GET /v1/news/categories/{category_id}

GET /v1/news/tags
GET /v1/news/tags/{tag_id}

GET /v1/news/sources
GET /v1/news/sources/{source_id}

GET /v1/news/creators
GET /v1/news/creators/{creator_id}

GET /v1/news/meta
GET /v1/news/search
GET /v1/news/timeline
GET /v1/news/feeds/rss
GET /v1/news/feeds/atom
```

```shell
docker build --target akasha -t akasha:latest .
docker build --target crawler -t akasha-crawler:latest .
```
