import json
import os
import sqlite3

db = os.path.join(os.environ["APPDATA"], "medquiz-app", "medquiz.db")
conn = sqlite3.connect(db)
conn.row_factory = sqlite3.Row
c = conn.cursor()

total = c.execute("SELECT COUNT(*) FROM questions").fetchone()[0]
no_subj = c.execute(
    "SELECT COUNT(*) FROM questions WHERE subjects IS NULL OR subjects = '' OR subjects = '[]'"
).fetchone()[0]
no_cite = c.execute(
    "SELECT COUNT(*) FROM questions WHERE citation_quote IS NULL OR citation_quote = ''"
).fetchone()[0]

print(f"Total: {total}")
print(f"Missing subjects: {no_subj}")
print(f"Missing citations: {no_cite}")
print("--- remaining gaps ---")
for row in c.execute(
    """SELECT topic, subjects, substr(citation_quote,1,90) as quote
       FROM questions
       WHERE subjects IS NULL OR subjects = '' OR subjects = '[]'
          OR citation_quote IS NULL OR citation_quote = ''
       ORDER BY topic"""
):
    print(dict(row))

print("--- sample filled ---")
for row in c.execute(
    """SELECT topic, subjects, substr(citation_quote,1,90) as quote
       FROM questions ORDER BY created_at DESC LIMIT 6"""
):
    print(dict(row))

conn.close()