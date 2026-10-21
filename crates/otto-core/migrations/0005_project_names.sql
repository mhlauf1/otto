-- A name Mike chooses for a project ("Brady Digital Consulting"), shown instead of the
-- folder name ("brady-digital"). NULL means "use the folder name". Only Otto's label
-- changes; the folder and repo are untouched.
ALTER TABLE repos ADD COLUMN display_name TEXT;
