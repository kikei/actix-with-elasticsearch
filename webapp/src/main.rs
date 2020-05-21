use actix_web::{
    get, web, App, HttpResponse, HttpServer, Responder
};
use elasticsearch::{
    Elasticsearch,
    IndexParts, GetParts, SearchParts, DeleteParts, UpdateParts,
    indices::IndicesCreateParts,
    http::{
        transport::{
            BuildError, SingleNodeConnectionPool,
            TransportBuilder
        }
    }
};
use futures::{future::TryFutureExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::Runtime;
use url::Url;

#[derive(Clone, Deserialize, Serialize, Debug)]
struct Onsen {
    #[serde(default)]
    pub id: Option<String>, // ID.
    pub area: String,       // 地域
    pub name: String,       // 施設名/旅館名
    pub address: String     // 住所
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct OnsenList {
    pub took: i32,
    pub onsens: Vec<Onsen>
}

#[derive(Deserialize, Debug)]
struct DocumentWithSource<S>
where S: Serialize
{
    _id: String,
    _index: String,
    _type: String,
    _source: S
}

#[derive(Deserialize, Debug)]
struct Document {
    _id: String,
    _index: String,
    _type: String
}

#[derive(Deserialize, Debug)]
struct SearchResultHits<S>
where S: Serialize
{
    hits: Vec<DocumentWithSource<S>>
}

/// {
///   "took": 3,
///   "timed_out": false,
///   "_shards": { "total": 1, "successful": 1, "skipped": 0, "failed": 0 },
///   "hits": {
///     "total": { "value": 7, "relation": "eq" },
///     "max_score": 1.0,
///     "hits": [
///       {
///         "_index": "onsen",
///         "_type": "_doc",
///         "_id": "_doc",
///         "_score": 1.0,
///         "_source" : {
///           "id": null,
///           "area": "東鳴子温泉",
///           "name": "初音旅館",
///           "address": "宮城県仙台市"
///         }
///       }
///     ]
///   }
/// }
#[derive(Deserialize, Debug)]
struct SearchResult<S>
where S: Serialize
{
    took: i32,
    hits: SearchResultHits<S>
}

#[derive(Debug, Deserialize)]
struct OnsenPath {
    id: String
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    query: Option<String>
}

fn create_elasticsearch_client(url: Url)
                               -> Result<Elasticsearch, BuildError>
{
    let conn_pool = SingleNodeConnectionPool::new(url);
    let transport = TransportBuilder::new(conn_pool).disable_proxy().build()?;
    Ok(Elasticsearch::new(transport))
}

type DBConnection = Elasticsearch;

#[get("/")]
async fn index(_conn: web::Data<DBConnection>) -> impl Responder
{
    format!("Let's try actix-web + elasticsearch!")
}

async fn search_onsen(conn: web::Data<DBConnection>,
                      query: web::Query<SearchQuery>) -> impl Responder {
    println!("search_onsen, query: {:?}", &query);
    let result = match query.query.as_ref().map(|s| s.as_str()) {
        None | Some("") =>
            conn.get_ref().search(SearchParts::Index(&["onsen"]))
            .send()
            .and_then(|r| async {
                r.json::<SearchResult<Onsen>>().await
            })
            .await,
        Some(qs) =>
            conn.get_ref().search(SearchParts::Index(&["onsen"]))
            .body(json!({
                "query": {
                    "multi_match": {
                        "query": qs,
                        "fields": ["name", "address"]
                    }
                }
            }))
            .send()
            .and_then(|r| async {
                r.json::<SearchResult<Onsen>>().await
            })
            .await
    };
    match result {
        Ok(result) => {
            println!("search result, took: {}, hits: {:?}",
                     &result.took, &result.hits);
            HttpResponse::Ok().json(OnsenList {
                took: result.took,
                onsens: result.hits.hits.iter().map(|d| Onsen {
                    id: Some(d._id.clone()),
                    ..d._source.clone()
                }).collect()
            })
        },
        Err(e) => {
            println!("Error in search_onsen: {}", &e);
            HttpResponse::NotFound().finish()
        }
    }
}

async fn get_onsen(conn: web::Data<DBConnection>,
                   path: web::Path<OnsenPath>) -> impl Responder {
    println!("get_onsen id: {}", &path.id);
    let result =
        conn.get_ref().get(GetParts::IndexId("onsen", path.id.as_str()))
        .send()
        .and_then(|r| async {
            r.json::<DocumentWithSource<Onsen>>().await.map(|r| Onsen {
                id: Some(r._id),
                ..r._source
            })
        })
        .await;
    match result {
        Ok(onsen) => {
            HttpResponse::Ok().json(&onsen)
        },
        Err(e) => {
            println!("Error in get_onsen: {}", &e);
            HttpResponse::NotFound().finish()
        }
    }
}

async fn create_onsen(conn: web::Data<DBConnection>,
                      data: web::Json<Onsen>) -> impl Responder
{
    println!("create_onsen, data: {:?}", &data);
    let mut onsen = data.into_inner();
    if onsen.id.is_some() {
        return HttpResponse::BadRequest().finish();
    }
    let parts = IndexParts::Index("onsen");
    println!("elasticsearch url: {}", &parts.clone().url());
    let result = conn.get_ref().index(parts).body(onsen.clone())
        .send()
        .and_then(|r| async { r.json::<Document>().await })
        .await;
    match result {
        Ok(result) => {
            println!("created onsen, result: {:?}", &result);
            onsen.id = Some(result._id);
            HttpResponse::Ok().json(&onsen)
        },
        Err(e) => {
            println!("failed to create onsen, error: {:?}", &e);
            HttpResponse::NotFound().finish()
        }
    }
}

async fn update_onsen(conn: web::Data<DBConnection>,
                      path: web::Path<OnsenPath>,
                      data: web::Json<Onsen>) -> impl Responder {
    println!("update_onsen id: {}, data: {:?}", &path.id, &data);
    let onsen = data.into_inner();
    if (&onsen.id).as_ref().filter(|id| id.to_string() == path.id).is_none() {
        println!("Id must match between url and body data");
        return HttpResponse::NotFound().finish();
    }
    let mut doc = onsen.clone();
    doc.id = None;
    let parts = UpdateParts::IndexId("onsen", path.id.as_str());
    println!("elasticsearch url: {}", &parts.clone().url());
    let result =
        conn.get_ref().update(parts)
        .body(json!({
            "doc": doc
        }))
        .send()
        .and_then(|r| async {
            r.text().await
        })
        .await;
    match result {
        Ok(result) => {
            println!("updated onsen, result: {:?}", &result);
            HttpResponse::Ok().json(&onsen)
        },
        Err(e) => {
            println!("Error in update_onsen: {}", &e);
            return HttpResponse::NotFound().finish()
        }
    }
}

async fn delete_onsen(conn: web::Data<DBConnection>,
                      path: web::Path<OnsenPath>) -> impl Responder {
    println!("delete_onsen id: {}", &path.id);
    let result =
        conn.get_ref().delete(DeleteParts::IndexId("onsen", path.id.as_str()))
        .send()
        .and_then(|r| async {
            r.json::<Document>().await
        })
        .await;
    match result {
        Ok(result) => {
            HttpResponse::Ok().json(json!({ "id": result._id }))
        },
        Err(e) => {
            println!("Error in delete_onsen: {}", &e);
            HttpResponse::NotFound().finish()
        }
    }
}

#[actix_rt::main]
async fn start(conn: DBConnection) -> std::io::Result<()>
{
    println!("start");
    HttpServer::new(move || {
        // For each worker
        println!("setup worker");
        App::new()
            .data(conn.clone())
            .service(index)
            .service(web::scope("/onsen")
                     .route("/", web::get().to(search_onsen))
                     .route("/", web::put().to(create_onsen))
                     .route("/{id}", web::get().to(get_onsen))
                     .route("/{id}", web::post().to(update_onsen))
                     .route("/{id}", web::delete().to(delete_onsen))
            )
    })
        .bind("0.0.0.0:8080")?
        .workers(2)
        .run()
        .await
}

async fn setup_index(esclient: Elasticsearch) {
    let result = esclient.indices()
        .create(IndicesCreateParts::Index("onsen_index"))
        .body(json!({
            "mappings": {
                "properties": {
                    "name": {
                        "type": "text",
                        "analyzer": "kuromoji"
                    },
                    "address": {
                        "type": "text",
                        "analyzer": "kuromoji"
                    }
                }
            }
        }))
        .send()
        .and_then(|r| async { r.text().await })
        .await;
    match result {
        Ok(res) => {
            println!("Successfully created an index: {}", &res)
        },
        Err(e) => {
            println!("Failed to setup index, error: {}", &e)
        }
    }
}

fn main() {
    let esclient =
        Url::parse("http://elasticsearch:9200")
        .map_err(|e| format!("Failed to parse url: {}", &e))
        .and_then(|url|
                  create_elasticsearch_client(url)
                  .map_err(|e| format!("Failed to create \
                                        elasticsearch client: {}", &e)));
    match esclient {
        Err(e) => {
            println!("Failed to parse url: {}", &e);
        },
        Ok(conn) => {
            Runtime::new().expect("").block_on(setup_index(conn.clone()));
            if let Err(e) = start(conn) {
                println!("Failed to start server: {}", &e);
            }
            println!("finished");
        }
    }
}
