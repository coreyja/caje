# Plan

## What did I do last time?

On this stream we got started looking at the Cache-Controler headers. Previously we cached the response no matter what.
But now we can use the Cache-Control header to determine if we should cache the response or not.
We opted to use the `http-cache-semantics` crate to parse the Cache-Control header for us, instead of implementing it ourselves.

We learned that Chrome was sending a `Max-Age: 0` header which was forcing our CDN to always refetch from the Origin.
NOTE: Use Firefox for future testing!

## Next Steps

- [ ] Cache to the FileSystem instead of holding everything in memory
- [ ] Add a `/admin/cache` endpoint to see what is in the cache
  - This will be important for debugging, especially when we get to Multinode
- [ ] Add a SQLite DB to store the cache metadata
  - This will be what we share between nodes. So it should be a "manifest" of the pages to cache
  - If we get a request for a page that is not in the DB, we should fetch it from the origin and add it to the DB
- [ ] Add LiteFS to share the Sqlite DB between nodes
  - This will be the "source of truth" for the manifest
  - We will use this to notify other nodes that a page exists
