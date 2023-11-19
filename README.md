# `caje`

`caje` is a caching reverse proxy developed live on stream by me, [coreyja](https://github.com/coreyja)

You can find a YouTube playlist containing all the previous streams at <https://www.youtube.com/playlist?list=PL0FtqJaYsqZ2v0FezJa15ynwBpo7KE8Xa>
And you can catch my live streams on my Twitch at <https://twitch.tv/coreyja>

## Overview

`caje` is a reverse proxy CDN. It sits inbetween the potentially slow origin server, and the users. It caches the responses from the origin server, and serves them to the users. `caje` respects `CacheControl` headers and only caches requests that contain caching headers.

`caje` plays middleman for all requests to the origin server, including those that are not cached.
You point the DNS for your domain to `caje`, and `caje` will forward the requests to the origin server.

`caje` is designed to be run in multiple regions around the world. When one node gets a request for a resource, it saves this information to a manifest that is shared between all nodes.

Currently there is an admin endpoint at `_caje/populate` that looks at this manifest and caches locally any files that are known to other nodes but not saved locally. In this way we can make sure all the nodes have all the cached content, so that requests from any region can be fast.
In the future this functionality will be moved to a background process that runs periodically, so the admin endpoint is no longer needed.

## Technical Details and Dependencies

`caje` is written in Rust and uses the [`axum`](https://github.com/tokio-rs/axum) Web Framework.
Axum provides the routing for our Admin routes, and a fallback route we use for proxying requests to the origin.

It is hosted on [fly.io](https://fly.io), and deployed to multiple regions. At the time of writing that is currently `ewr` and `lhr`, but will likely be expanded to more regions in the future.

The admin endpoints use [`maud`](https://github.com/lambda-fairy/maud) for templating. Maud provides an `html!` macro that we use to construct the HTML responses. This integrates with axum so we can return the response from `html!` in axum routes.

These admin endpoints are currently UNAUTHENTICATED. Eventually these will be locked down to only allow requests authorized admins. Ideally this authentication will be implemented with Passkeys.

`caje` uses ['http_cache_semantics'](https://github.com/kornelski/rusty-http-cache-semantics) for interpretting the caching headers, and determining if a request and response should or should not be cached.

`caje` uses [`cacache`](https://github.com/zkat/cacache-rs) to implement it's File System cache. This cache is specific to the individual node. It currently does NOT survive server reboots/deploys. This will be fixed in the future, by moving the cache directory to a shared volume that persists between deploys.

`caje` uses [`sqlite`](https://www.sqlite.org/index.html) and [`litefs`](https://github.com/superfly/litefs) for the DB Manifest. This is stored as a Sqlite DB locally on each node, and is syncronized between nodes by `litefs`. This DB is used to keep track of which files are cached on which nodes, so that we can populate the cache on each node with the files that are cached on other nodes.
We utitlize the `litefs` HALT mechanism to allow writing to the shared DB from replica nodes. This reduces the theoretical throughput of the database when writing from replicas, but should be fine for our use case.

## Admin Endpoints

The following admin endpoints exist to help with managing the cache, and debugging `caje`. They are currently authenticated, with a shared password that is set as an environment variable.

We'd like to change this to use Passkeys in the future.

- `GET#_caje/list` Displays the current values in both the FileSystem cache and the DB Manifest
- `POST#_caje/clear_fs` Clears the File System cache on the node that recieves this request
- `POST#_caje/clear_db` Clears the DB Manifest which is shared between all nodes
- `POST#_caje/populate` Checks the manifest for any pages that are not cached locally, and caches them to the File System

- `GET#_caje/auth` Displays the Admin Login Page
- `POST#_caje/auth` Login to the Admin Dashboard

## Acknowledgements

Special thanks for `TCP Stream` for coming up with the name `caje`! This is a play on my initials `cja` and is pronounced like `cache`.
Love the name, thanks so much TCP Stream!
