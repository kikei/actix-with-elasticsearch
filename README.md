# actix-with-elasticsearch
Simple application for testing actix with elasticsearch

I made this web application for preliminary survey of actix-web and elasticsearch with japanese full-text search.

For more information, check below link:

https://kikei.github.io/rust/2020/05/13/actixweb-elasticsearch.html

## How to

You can launch app by just running docker-compose.

```sh
docker-compose up -d
```

## APIs

Craete new document:

```sh
curl -XPUT -H "Content-Type: application/json" --data \
  '{ "name": "西多賀旅館", "address": "宮城県大崎市鳴子温泉新屋敷78-3", "area": "鳴子温泉" }' \
   http://localhost:8080/onsen/
```

Get document by id:

```sh
url http://localhost:8080/onsen/umbINHIB3Vl9TKW-8SVx
```

Update a document with id:

```sh
curl -XPOST -H "Content-Type: application/json" --data \
  '{ "id": "umbINHIB3Vl9TKW-8SVx", "name": "西多賀旅館 東北宮城の湯治宿", "address": "宮城県大崎市鳴子温泉新屋敷78-3", "area": "鳴子温泉" }' http://localhost:8080/onsen/umbINHIB3Vl9TKW-8SVx
``` 

Delete a document by id:

```sh
curl -XDELETE http://localhost:8080/onsen/umbINHIB3Vl9TKW-8SVx
``` 

List all documents:

```sh
curl http://localhost:8080/onsen
```

Search document by query:

```sh
curl "http://localhost:8080/onsen/?query=%E6%B8%A9%E6%B3%89"
```


