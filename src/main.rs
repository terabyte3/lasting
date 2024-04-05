use actix_web::{get, http, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use cached::proc_macro::cached;
use env_logger;
use reqwest::Client;
use serde_json::{json, Value, from_str};
use std::env;
mod types;

#[cached(
    size = 1000,
    option = true,
    key = "String",
    convert = r#"{format!("{}", mbid)}"#
)]
async fn get_album_cover(
    http: &Client,
    mbid: &String,
) -> Option<types::CoverArt> {
    let cover_res = http.get(format!("https://coverartarchive.org/release/{}", mbid)).send().await;
    if cover_res.is_err() {
        return None
    };
    let cover_res = cover_res.unwrap();

    if cover_res.status() != 200 {
        return None
    };

    let cover_art_result = cover_res.json::<types::CoverArt>().await;
    if cover_art_result.is_err() {
        return None
    };

    let cover_art = cover_art_result.unwrap();

    if cover_art.images.is_empty() {
        return None
    }
    
    Some(cover_art)

}

#[cached(
    size = 1000,
    time = 86400,
    time_refresh = true,
    key = "String",
    convert = r#"{ format!("{}", u) }"#,
    result = true
)]
async fn get_top_albums(
    http: &Client,
    u: String,
) -> Result<Vec<types::Album>, Box<dyn std::error::Error>> {
    let top_albums_res = http
        .get("https://ws.audioscrobbler.com/2.0/")
        .query(&[
            ("method", "user.getTopAlbums"),
            ("api_key", env::var("LASTFM_KEY").unwrap().as_str()),
            ("format", "json"),
            ("period", "7day"),
            ("limit", "5"),
            ("user", &u),
        ])
        .send()
        .await?;
    
    if top_albums_res.status() != 200 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("user {} not found", u),
        )));
    }

    let top_albums_json: types::TopAlbums = match from_str(&top_albums_res.text().await.unwrap_or("{}".to_string())) {
        Ok(data) => data,
        Err(_) => types::TopAlbums { data: types::TopAlbumsData {albums: Vec::new()}},
    };
    if top_albums_json.data.albums.len() < 1 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("no listening data for user {}", u),
        )));
    }
    let mut final_albums: Vec<types::Album> = Vec::with_capacity(5);
    for mut album in top_albums_json.data.albums {
        if album.mbid.is_empty() {
            continue
        }
        album.cover_art = get_album_cover(http, &album.mbid).await;
        final_albums.push(album);
    }
    
    Ok(final_albums)
}


#[cached(
    size = 1000,
    time = 86400,
    time_refresh = true,
    key = "String",
    convert = r#"{ format!("{}", u) }"#,
    result = true
)]
async fn get_top_tracks(
    http: &Client,
    u: String,
) -> Result<types::LastFMResponse, Box<dyn std::error::Error>> {
    let top_artists_res = http
        .get("https://ws.audioscrobbler.com/2.0/")
        .query(&[
            ("method", "user.getTopArtists"),
            ("api_key", env::var("LASTFM_KEY").unwrap().as_str()),
            ("format", "json"),
            ("period", "7day"),
            ("limit", "1"),
            ("user", &u),
        ])
        .send()
        .await?;
    
    if top_artists_res.status() != 200 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("user {} not found", u),
        )));
    }
    let artist_json_result = top_artists_res.json::<types::TopArtists>().await;
    let top_artists: types::TopArtists = match artist_json_result {
        Ok(data) => data,
        Err(_) => types::TopArtists { topartists: types::TopArtistsData {artist: Vec::new()} },
    };
    let top_artist = if top_artists.topartists.artist.len() > 0 {
        Some(top_artists.topartists.artist[0].to_owned())
    } else {
        None
    };
    if top_artist.is_none() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("no listening data for user {}", u),
        )));
    }
    let top_tracks_res = http
        .get("https://ws.audioscrobbler.com/2.0/")
        .query(&[
            ("method", "user.getTopTracks"),
            ("api_key", env::var("LASTFM_KEY").unwrap().as_str()),
            ("format", "json"),
            ("period", "7day"),
            ("limit", "48"),
            ("user", &u),
        ])
        .send()
        .await
        .unwrap();
    let mut top_tracks: types::TopTracks = top_tracks_res.json::<types::TopTracks>().await.unwrap();
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
    Ok(types::LastFMResponse {
        top_artist,
        top_tracks: top_tracks.toptracks.track,
    })
}

#[get("/")]
async fn index() -> Result<HttpResponse, http::Error> {
    Ok(HttpResponse::Ok().body("Hello World!"))
}

#[get("/{user}")]
async fn user(data: web::Data<types::AppState>, user: web::Path<String>) -> impl Responder {
    let res = get_top_tracks(&data.client, user.to_string()).await;
    if res.is_ok() {
        return HttpResponse::Ok().json(res.unwrap());
    } else {
        return HttpResponse::InternalServerError().body(res.err().expect("Error").to_string());
    }
}
#[get("/{user}/albums")]
async fn top_albums(data: web::Data<types::AppState>, album_user: web::Path<String>) -> impl Responder {
    let res = get_top_albums(&data.client, album_user.to_string()).await;
    if res.is_ok() {
        return HttpResponse::Ok().json(res.unwrap());
    } else {
        return HttpResponse::InternalServerError().body(res.err().expect("Error").to_string());
    }
}

#[get("/{user}/current")]
async fn current(data: web::Data<types::AppState>, current_user: web::Path<String>) -> impl Responder {
    let http = &data.client;
    let now_playing_res = http
        .get("https://ws.audioscrobbler.com/2.0/")
        .query(&[
            ("method", "user.getRecentTracks"),
            ("api_key", env::var("LASTFM_KEY").unwrap().as_str()),
            ("format", "json"),
            ("user", &current_user.to_string()),
            ("limit", "1"),
        ])
        .send()
        .await
        .unwrap();
    HttpResponse::Ok().json(now_playing_res.json::<Value>().await.unwrap())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let key = env::var("LASTFM_KEY");
    if key.is_err() {
        panic!("no lastfm key provided!");
    }
    println!("Starting on port {}", 3000);
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(types::AppState {
                client: Client::new(),
            }))
            .wrap(Logger::default())
            .service(user)
            .service(index)
            .service(current)
            .service(top_albums)
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
}
