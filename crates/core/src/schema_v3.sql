-- cache.db, version 3: previews of the web links on canvases, fetched from the pages.

-- One row per URL fetched. A failed fetch keeps a row with its error, so it waits before the
-- next try. image and favicon are file names in .neoworks-projects/previews/; fetched is Unix ms.
CREATE TABLE link_previews (
  url TEXT PRIMARY KEY,
  title TEXT,
  description TEXT,
  site TEXT,
  image TEXT,
  favicon TEXT,
  error TEXT,
  fetched INTEGER NOT NULL
);
