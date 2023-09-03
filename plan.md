# Plan

## What did I do last time?

- [x] Cache to the FileSystem instead of holding everything in memory
- [x] Add a `/_caje/list` endpoint to see what is in the cache
  - This will be important for debugging, especially when we get to Multinode
- [x] Add a SQLite DB to store the cache metadata

  - This will be what we share between nodes. So it should be a "manifest" of the pages to cache
  - If we get a request for a page that is not in the DB, we should fetch it from the origin and add it to the DB

  We moved the cache from memory to the File System. To do this we needed to serialize the objects and be able to deserialize them. We went with `postcard` as the serialization format/library. This uses `serde` so we created Structs that hold all the request and response information we need and can be serialized.

  The admin endpoint is very simply and currently only looks at the cache. We should expand it to also show information about the DB, and if everything in the manifest is already cached.

## Next Steps

- [ ] Deploy our slow example server somewhere
- [ ] Deploy single node (Probably to fly.io)
- [ ] Deploy to multiple nodes (Probably to fly.io)
- [ ] Add LiteFS to share the Sqlite DB between nodes
  - This will be the "source of truth" for the manifest
  - We will use this to notify other nodes that a page exists
- [ ] Create Endpoint to fetch any missing pages from the origin
  - This will be used to populate the cache
- [ ] Move the cache population to a seperate process that runs in the background
- [ ] Move some hard coded proxy information to config file
- [ ] Allow proxying to multiple origins

## History

### First Stream

On this stream we got started looking at the Cache-Controler headers. Previously we cached the response no matter what.
But now we can use the Cache-Control header to determine if we should cache the response or not.
We opted to use the `http-cache-semantics` crate to parse the Cache-Control header for us, instead of implementing it ourselves.

We learned that Chrome was sending a `Max-Age: 0` header which was forcing our CDN to always refetch from the Origin.
NOTE: Use Firefox for future testing!
