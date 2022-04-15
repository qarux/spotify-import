use clap::Parser;
use lofty::{Accessor, Tag};
use log::{debug, error, info, warn};
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::model::{PlayableId, SearchResult, SearchType, TrackId};
use rspotify::{scopes, AuthCodeSpotify, ClientResult, Credentials, OAuth};
use walkdir::WalkDir;

/// Simple program to import your local music library to Spotify
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the directory with music
    #[clap(short, long)]
    path: String,

    /// Spotify Client ID
    #[clap(short, long)]
    client_id: String,

    /// Spotify Client Secret
    #[clap(short, long)]
    secret: String,
}

struct SearchQuery(String);

impl TryFrom<Tag> for SearchQuery {
    type Error = ();

    fn try_from(tag: Tag) -> Result<Self, Self::Error> {
        match (tag.title(), tag.artist()) {
            (Some(title), Some(artist)) => Ok(SearchQuery(format!("{} - {}", title, artist))),
            _ => Err(()),
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "spotify_import=info"),
    );

    let args = Args::parse();
    let spotify = auth(&args.client_id, &args.secret).await;

    let tags = collect_track_tags(&args.path);
    info!("Found {} local tracks", tags.len());

    let queries = tags
        .into_iter()
        .filter_map(|tag| SearchQuery::try_from(tag).ok())
        .collect();
    let track_ids = get_track_ids(queries, &spotify).await;
    info!("Found {} tracks in the Spotify library", track_ids.len());

    let result = add_tracks_to_spotify(spotify, "Imported", &track_ids).await;
    match result {
        Ok(_) => info!("Successfully imported {} tracks", track_ids.len()),
        Err(_) => error!("Failed to import tracks"),
    }
}

async fn auth(id: &str, secret: &str) -> AuthCodeSpotify {
    let creds = Credentials::new(id, secret);
    let oauth = OAuth {
        redirect_uri: "http://localhost:8888/callback".to_string(),
        scopes: scopes!("playlist-modify-private"),
        ..Default::default()
    };
    let mut spotify = AuthCodeSpotify::new(creds, oauth);
    let url = spotify.get_authorize_url(false).unwrap();

    info!("Obtaining the access token");
    spotify.prompt_for_token(&url).await.unwrap();

    spotify
}

fn collect_track_tags(dir: &str) -> Vec<Tag> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| lofty::read_from_path(e.path(), false).ok())
        .filter_map(|file| {
            file.primary_tag()
                .cloned()
                .or_else(|| file.first_tag().cloned())
        })
        .collect()
}

async fn get_track_ids(queries: Vec<SearchQuery>, spotify: &AuthCodeSpotify) -> Vec<TrackId> {
    let mut track_ids = vec![];
    for query in queries {
        let result = spotify
            .search(&query.0, &SearchType::Track, None, None, Some(1), None)
            .await;
        if let Ok(SearchResult::Tracks(track)) = result {
            if let Some(id) = track.items.first().and_then(|t| t.id.clone()) {
                track_ids.push(id);
                
                continue;
            }
        }

        warn!("'{}' not found in the Spotify library", query.0);
    }
    track_ids
}

async fn add_tracks_to_spotify(
    spotify: AuthCodeSpotify,
    playlist_name: &str,
    track_ids: &[TrackId],
) -> ClientResult<()> {
    let user_id = spotify.current_user().await.expect("").id;
    let playlist = spotify
        .user_playlist_create(&user_id, playlist_name, Some(false), Some(false), None)
        .await?;

    // A maximum of 100 items can be added in one request
    let mut position = 0;
    for chunk in track_ids.chunks(100) {
        let items: Vec<&dyn PlayableId> = chunk.iter().map(|id| id as &dyn PlayableId).collect();

        spotify
            .playlist_add_items(&playlist.id, items, Some(position))
            .await?;
        position += 100;
        debug!("Imported 100 tracks at position {}", position)
    }

    Ok(())
}
