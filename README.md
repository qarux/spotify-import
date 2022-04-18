# spotify-import
Simple program to import your local music library to Spotify.

## Usage
1. Create an app on [Spotify for Developers](https://developer.spotify.com/dashboard/applications)
2. Add the redirect URI http://localhost:8888/callback
3. [Download](https://github.com/qarux/spotify-import/releases/latest) the binary for your platform.
4. Run spotify-import with the `client id` and the `secret` from your application's page.
````
spotify-import -p <path to music> -c <client id> -s <secret>
````
5. Tracks will be saved in the "Imported" playlist on Spotify.
