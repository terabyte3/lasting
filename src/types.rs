use reqwest::Client;
use serde::{Serialize, Deserialize};
use serde_json::Value;

pub struct AppState {
    pub client: Client,
}

#[derive(Serialize, Clone, Debug)]
pub struct LastFMResponse {
    pub top_artist: Option<TopArtist>,
    pub top_tracks: Vec<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopArtists {
    pub topartists: TopArtistsData,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopArtistsData {
    pub artist: Vec<TopArtist>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopArtist {
    pub name: String,
    pub url: String,
    pub image: Vec<Image>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Image {
    pub size: String,
    #[serde(rename = "#text")]
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopTracks {
    pub toptracks: TopTracksData,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopTracksData {
    pub track: Vec<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopTrack {
    pub name: String,
    pub image: Vec<Image>,
    pub artist: TopArtist,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct TopAlbums {
    #[serde(rename="topalbums")]
    pub data: TopAlbumsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopAlbumsData {
    #[serde(rename="album")]
    pub albums: Vec<Album>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
pub struct Album {
    pub artist: Artist,
    pub mbid: String,
    pub url: String,
    pub cover_art: Option<CoverArt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub url: String,
    pub name: String,
    pub mbid: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverArt {
    pub images: Vec<CoverArtImage>,
    pub release: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Deserialize)]
pub struct CoverArtImage {
    pub back: bool,
    pub front: bool,
    pub id: i64,
    pub image: String,
    pub thumbnails: Thumbnails,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Thumbnails {
    #[serde(rename = "1200")]
    pub n1200: String,
    #[serde(rename = "250")]
    pub n250: String,
    #[serde(rename = "500")]
    pub n500: String,
    pub large: String,
    pub small: String,
}