---
title: Backups, export and import
description: How to save a backup of your library, restore it, and move your list to or from a spreadsheet.
sidebar:
  order: 12
---

Your libraries live on your phone or computer. If you lose it, it breaks or you delete things by mistake, you only get back what you have in a backup. It is a good idea to make them from time to time.

To start, open the menu (on a phone, the three bars at the top left) and go to "Backups".

![The "Backups" dialog. (Screenshot from the Spanish app.)](../../es/using-the-app/backups-dialog.webp)

## Save a backup

A backup is a file with your books and their covers.

1. Go to "Backups".
2. Leave "Protect with a password" on. It is on already.
3. Type a password of at least 8 characters and repeat it.
4. Tap "Save a backup of" and the name of your library.
5. Choose where to save the file: a folder, a USB stick or your Google Drive, for example.

When it finishes, the app says how many books, copies and covers it saved.

**Write the password down.** Without it nobody can open the backup, and the app does not keep it anywhere. Keep it on paper or in the password program you use.

You can remove the password with the "Protect with a password" switch. The app warns you that anyone who finds the file will be able to read it: the names, what you own and who has what.

## Save all your libraries at once

If you have more than one library, the "Back up the whole device" button saves them all in a single file.

## Restore a backup

1. Go to "Backups" and tap "Restore backup".
2. Choose the file.
3. If it has a password, type it and tap "Unlock and restore". If you get it wrong, nothing is restored and you can try again.

Each library in the file comes back with its name. The ones you already have on your device with books in them are left alone.

The backup also keeps the [trash](../trash/). What comes back in it gets 30 more days from when you restore, even if the backup is old, and the summary says how many things it brings.

**One thing worth knowing.**

- If you restore the backup on the same device that made it, the library comes back as it was.
- If you restore it on another device, it appears as a new library with the same books, and it **does not join the original**.

To have the original library on a new device, pass it from another device that has it (see [Share at home](../share-at-home/)) or download it from your account, if you have one.

If the app says the backup was made with another version of the app, update the app and try again.

## Your list in a spreadsheet

On the same screen, the "Spreadsheet" part lets you move your list to Excel, LibreOffice or a similar program, or bring in a list you already have.

### Export

Tap "Export CSV" and choose where to save it. A CSV is a file that Excel and other programs can open, with one line for each copy (each one you have of something, on paper or digital). It carries every kind together, books, games, films…, each with its kind in the first column. The app says how many lines it wrote.

The CSV **has no password**: anyone who finds it can read it. It also does not carry everything: it does not save the covers, nor sagas and collections, nor what you marked as read, nor the stars. To keep everything, use the backup.

### Import

Tap "Import CSV" and choose the file. The first line of the file must be the names of the columns, written exactly like this. Only `title` is needed; the other columns may be missing, and one that does not apply to that line's kind stays empty:

| Column | What it holds |
|---|---|
| `kind` | The kind, with one of the words in the table below. Empty, it is a book |
| `title` | The title. **It is the only required one.** |
| `subtitle` | The subtitle |
| `authors` | Who wrote it. If there are several people, separate them with a semicolon: `Margaret Weis; Tracy Hickman` |
| `writers`, `illustrators` | Writing and art, for comics |
| `designers` | Design, for board games |
| `directors` | Direction, for films and series |
| `artists` | Artist, for music |
| `studios` | Studio, for video games |
| `publisher` | The publisher (or the label, the distributor…) |
| `published_year` | The year, in digits |
| `language` | The language, with its two-letter code: `es` for Spanish, `en` for English… |
| `isbn13` | The ISBN, the number every book has next to its barcode |
| `barcode` | The barcode of what has no ISBN (EAN or UPC) |
| `number` | The issue number, for comics |
| `players_min`, `players_max`, `min_age` | Players and minimum age, for board games |
| `duration_minutes` | The length in minutes, for board games and films |
| `seasons` | How many seasons a series has |
| `genres` | The genres, separated by semicolons, with the app's codes (export a CSV to see them) |
| `tags` | The tags, separated by semicolons |
| `format` | The format, with one of the words in the table below |
| `owner` | Whose it is |
| `quantity` | How many items. If you put nothing, one |
| `location` | Where it is |
| `acquired_on` | The purchase date, like `2026-10-16` |
| `platform` | A video game's platform |
| `copy_seasons` | Which seasons that copy of a series holds, like `1-3` |

In the `kind` column, these words:

| In the app | In the CSV |
|---|---|
| Book | `book`, or leave it empty |
| Comic | `comic` |
| Board game | `boardgame` |
| Film | `film` |
| Series | `tv_series` |
| Video game | `videogame` |
| Music | `music` |
| Other | `other` |

In the `format` column, one of that kind's formats:

| In the app | In the CSV |
|---|---|
| Hardcover · Paperback · Pocket | `hardcover` · `paperback` · `pocket` |
| Ebook · PDF · Audiobook | `ebook` · `pdf` · `audiobook` |
| Single issue (comic) | `single_issue` |
| Box (board game) | `box` |
| DVD · Blu-ray · 4K | `dvd` · `bluray` · `uhd` |
| Physical (video game) | `physical` |
| Vinyl · CD · Cassette | `vinyl` · `cd` · `cassette` |
| Digital | `digital` |
| Other, or no format | `other`, or leave it empty |

What happens when you import:

- Each line comes in with its kind; without the `kind` column (as in the CSV files the app exported before), everything comes in as a book, and you can change the kind afterwards from its page.
- If the ISBN on a line is already in your library, it is not repeated: a copy is added to it. What has no ISBN (a game, a film…) always comes in as new, so importing the same file twice duplicates it.
- Lines with errors are skipped and the rest is imported. When it finishes, the app says how many new things there are, to how many you already had a copy was added, and how many lines were rejected, with the number of each.
- If there is a column the app does not know, it ignores it.

You can try first with a new library, to see how it turns out before you mix it with yours.
