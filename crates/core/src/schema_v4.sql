-- cache.db, version 4: a link preview whose page loaded keeps why its image or icon didn't, so
-- it is fetched again after the same wait as a failed page.
ALTER TABLE link_previews ADD COLUMN image_error TEXT;
