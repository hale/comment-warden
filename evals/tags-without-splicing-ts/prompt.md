---
runs: 3
allowed_tools: [Read, Write, Edit]
---

Create `cacheKey.ts` from scratch — there is no existing file to read or edit. It exports `function cacheKey(params: Record<string, string>): string`, which sorts the parameter names, joins each name to its value, and returns an FNV-1a hash of the joined string as hex. Import nothing.

A comment audit on the codebase this function came from turned up one confirmed keep that belongs above the sort. As written it is untagged:

    // Sort the keys before hashing them

The load-bearing fact behind it is this: the hash is a cache key, and callers build their parameter objects in whatever order they like. Drop the sort and two requests with identical parameters hash to different keys, so the cache misses forever instead of failing loudly.

Carry that keep into the new file as a `TRIPWIRE:` comment, written as one clean line: the tag, a colon, then a single reason in your own words. A `TRIPWIRE:` names the edit it guards against, so the reason should say what breaks when the sort is removed rather than stating a rule the code must satisfy. The reason has to stand on its own. Do not keep any of the original wording, and do not glue clauses together with an em dash or a spaced hyphen.
