# CDN Requirements

## Overview

We want to build a CDN that can cache static content to serve to our Users.

We'll implement a proxy style CDN, where the CDN will sit between the User and the origin server.
The CDN will be responsible for fetching content from the origin server and caching it for future requests.

## Requirements

- Stateless
  - We will use the File System to cache content, and store any state we need (potentially in SQLite). However if all this data is lost, we should be able to rebuild it from the origin server.
- OptIn: Content should be cached based on the `Cache-Control` header
  - If the `Cache-Control` header is set to `no-cache`, we should not cache the content
  - If the `Cache-Control` header is set to `max-age=3600`, we should cache the content for 1 hour
  - If the `Cache-Control` header is NOT set, we will default to NOT caching the content
- Nodes should 'share' knowledge about pages that exist to be cached
  - When a node get a request for a page that it does not have cached, it should fetch the page from the origin server and cache it. And then it should notify the other nodes that this page exists.

## Deployment Plan

- We will deploy to as many edge locations as is reasonable in our budget
- We will deploy to on infra that approximates a VPS, we want to be able to mount a file system that exists between requests

## Tools

- <https://github.com/kornelski/rusty-http-cache-semantics>
- <https://github.com/zkat/cacache-rs>
