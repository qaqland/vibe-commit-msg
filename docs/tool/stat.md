Show staged file change statistics. Returns a tab-separated list with
added/deleted line counts for each changed file in the format
`+ADD\t-DEL\t/PATH`. Binary files show `++\t--`.

If no staged changes exist, returns `[No changes found]`. Output is capped
at 200 entries and 50 KB.
