use actix_web::{get, http, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use cached::proc_macro::cached;
use env_logger;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;

struct AppState {
    client: Client,
}

#[derive(Serialize, Clone, Debug)]
struct LastFMResponse {
    top_artist: TopArtist,
    top_tracks: Vec<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TopArtists {
    topartists: TopArtistsData,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TopArtistsData {
    artist: Vec<TopArtist>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TopArtist {
    name: String,
    url: String,
    image: Vec<Image>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Image {
    size: String,
    #[serde(rename = "#text")]
    text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TopTracks {
    toptracks: TopTracksData,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TopTracksData {
    track: Vec<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct TopTrack {
    name: String,
    image: Vec<Image>,
    artist: TopArtist,
    url: String,
}

#[cached(
    size = 1000,
    time = 86400,
    time_refresh = true,
    key = "String",
    convert = r#"{ format!("{}", u) }"#,
    result = true
)]
async fn get_lastfm(
    http: &Client,
    u: String,
) -> Result<LastFMResponse, Box<dyn std::error::Error>> {
    let top_artists_res = http
        .get("https://ws.audioscrobbler.com/2.0/")
        .query(&[
            ("method", "user.getTopArtists"),
            ("api_key", env::var("LASTFM_KEY").unwrap().as_str()),
            ("format", "json"),
            ("period", "1month"),
            ("limit", "1"),
            ("user", &u),
        ])
        .send()
        .await?;
    if top_artists_res.status() != 200 {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("user {} not found", u))))
    }
    let top_artists: TopArtists = top_artists_res.json::<TopArtists>().await.unwrap();
    let top_tracks_res = http
        .get("https://ws.audioscrobbler.com/2.0/")
        .query(&[
            ("method", "user.getTopTracks"),
            ("api_key", env::var("LASTFM_KEY").unwrap().as_str()),
            ("format", "json"),
            ("period", "1month"),
            ("limit", "48"),
            ("user", &u),
        ])
        .send()
        .await
        .unwrap();
    let mut top_tracks: TopTracks = top_tracks_res.json::<TopTracks>().await.unwrap();
    for trackn in 0..top_tracks.toptracks.track.len() {
        let track = &mut top_tracks.toptracks.track[trackn];
        if track.get("image").unwrap().as_array().unwrap()[3]
            .as_object()
            .unwrap()
            .get("#text")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("2a96cbd8b46e442fc41c2b86b821562f")
        {
            // i sincerely apologize for writing this, and for the fact that you had to read it
            let cur_track_info = http
                .get("https://ws.audioscrobbler.com/2.0/")
                .query(&[
                    ("method", "track.getInfo"),
                    ("api_key", env::var("LASTFM_KEY").unwrap().as_str()),
                    ("format", "json"),
                    (
                        "artist",
                        track
                            .get("artist")
                            .unwrap()
                            .get("name")
                            .unwrap()
                            .as_str()
                            .unwrap(),
                    ),
                    ("track", track.get("name").unwrap().as_str().unwrap()),
                ])
                .send()
                .await
                .unwrap();
            let ctinfo = cur_track_info.json::<Value>().await.unwrap();
            let a = if ctinfo.get("track").unwrap().get("album").is_some() {
                ctinfo.get("track").unwrap().get("album").unwrap()
            } else {
                ctinfo.get("track").unwrap()
            };
            *track.get_mut("image").unwrap() = json!(a.get("image").unwrap_or(&json!(null)));
            top_tracks.toptracks.track[trackn] = track.clone();
        }
    }
    Ok(LastFMResponse {
        top_artist: top_artists.topartists.artist[0].clone(),
        top_tracks: top_tracks.toptracks.track,
    })
}

#[get("/")]
async fn index() -> Result<HttpResponse, http::Error> {
    Ok(HttpResponse::Ok().body("Hello World!"))
}

#[get("/{user}")]
async fn user(data: web::Data<AppState>, user: web::Path<String>) -> impl Responder {
    let res = get_lastfm(&data.client, user.to_string()).await;
    if res.is_ok() {
        return HttpResponse::Ok().json(res.unwrap());
    } else {
        return HttpResponse::InternalServerError().body(res.err().expect("Error").to_string());
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    println!("Starting on port {}", 8080);
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(AppState {
                client: Client::new(),
            }))
            .wrap(Logger::default())
            .service(user)
            .service(index)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
