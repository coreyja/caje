# Plan

## What did I do last time?

### Sixth Stream

Today we got LiteFS Halt mode working!!

The last two puzzle pieces were to use the Open File Descriptor variants of file locking.
And to keep the File Descriptor open for the duration of HALT process

Now we can do read and writes from _any_ node in our Cluster :tada:

This means that when a replica gets a request for something it wants to cache, it can use the HALT
to aquire a write lock and do its write. Then it releases the lock and allows other nodes to continue writing.
Its NOT recommended to do this for write heavy applications, but for our use case it should be fine.
Plus I kinda want to stress test the HALT functionality to learn where its limits are.

## Next Steps

- [x] `_caje/list` should return the TTL of pages in the cache
- [x] Create Endpoint to clear the File System Cache
- [x] Create Endpoint to clear the SQLite Cache
- [x] Make sure DB doesn't record duplicate entries
- [ ] Create Endpoint to fetch any missing pages from the origin
  - This will be used to populate the cache
  - If there are things in Sqlite that are not in the File System, we should fetch them from the origin
- [ ] Move some hard coded proxy information to config file
- [ ] Allow proxying to multiple origins
- [ ] Move the cache population to a seperate process that runs in the background
- [ ] Make `_caje` endpoints require some kind of authentication
- [ ] Move the cache dir to somewhere persisted in the Fly.io VM
- [ ] Write an awesome Readme.md
- [ ] Cleanup and Publish `litefs-rs` to crates.io

## History

### First Stream

On this stream we got started looking at the Cache-Controler headers. Previously we cached the response no matter what.
But now we can use the Cache-Control header to determine if we should cache the response or not.
We opted to use the `http-cache-semantics` crate to parse the Cache-Control header for us, instead of implementing it ourselves.

We learned that Chrome was sending a `Max-Age: 0` header which was forcing our CDN to always refetch from the Origin.
NOTE: Use Firefox for future testing!

### Third Stream

- [x] Cache to the FileSystem instead of holding everything in memory
- [x] Add a `/_caje/list` endpoint to see what is in the cache
  - This will be important for debugging, especially when we get to Multinode
- [x] Add a SQLite DB to store the cache metadata

  - This will be what we share between nodes. So it should be a "manifest" of the pages to cache
  - If we get a request for a page that is not in the DB, we should fetch it from the origin and add it to the DB

  We moved the cache from memory to the File System. To do this we needed to serialize the objects and be able to deserialize them. We went with `postcard` as the serialization format/library. This uses `serde` so we created Structs that hold all the request and response information we need and can be serialized.

  The admin endpoint is very simply and currently only looks at the cache. We should expand it to also show information about the DB, and if everything in the manifest is already cached.

### Fourth Stream

In our Fourth stream we got everything deployed to Fly.io

We started by deploying `slow_server` to Paris.
We then depolyed `caje` to New Jersey and London, and have it proxying to the `slow_server` in Paris.

We got LiteFS working for `caje` so that the replicas can read the SQLite and the primary can write to it.
We currently blow up if the replicas try to write to the DB, fixing this is up next!

### Fifth Stream

We tried to implement the LiteFS HALT mechanism. We used <https://github.com/superfly/litefs-go/blob/main/litefs.go> as our
reference implementation.

We got the code in a spot where we _expected_ it to work, but haven't succesfully gotten it working in Fly (or tried locally).

We ~are currently~ WERE getting an `errno: 38` which we believe means `ENOSYS` or `Function not implemented`. Which is _weird_ cause Fly.io makes LiteFS so it should work on their platform. But as I type this it might be an OS thing, but also strange debain wouldn't support it.

This might have been fixed by changing the lock cmd to `FcntlCmd::SetLockWait`. BUTT this doesn't seem like its actually making the lock correctly. Or maybe litefs isn't reading it correctly?

We are still getting this error from the replica: `error returned from database: (code: 1544) attempt to write a readonly database`
The Primary doesnt have this issue since it doesn't _need_ HALTing to work to write to the DB
