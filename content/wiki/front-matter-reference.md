+++
title = "front matter reference"
description = "all available front matter fields"
tags = ["documentation"]
category = "documentation"
+++

# front matter reference

Every content file begins with a TOML block delimited by `+++`.

## posts

```toml
+++
title = "entry title"         # required
date = "2024-03-15"           # YYYY-MM-DD
tags = ["tag1", "tag2"]
draft = false                 # true = excluded from build
pinned = false                # true = floated to top of feed
slug = "custom-url"           # overrides filename-derived slug
description = "short summary" # used in meta tags
+++
```

## wiki pages

```toml
+++
title = "page title"          # required
description = "one liner"     # shown in wiki index
tags = ["tag1"]
category = "general"          # groups pages in wiki index
updated = "2024-03-01"        # display-only date
draft = false
slug = "custom-url"
+++
```

## standalone pages

```toml
+++
title = "about"               # required
draft = false
in_nav = false                # future: auto-add to nav
description = "optional"
slug = "about"
+++
```

See [[getting started]] for the broader setup guide.
