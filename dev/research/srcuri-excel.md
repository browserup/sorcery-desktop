Short answer: **Desktop Excel doesn’t have a command-line or ms-excel URI option to jump to a sheet/cell/range.** You can pass a file to `excel.exe`, but there’s no documented switch for “open at A1” (Excel’s switches are things like `/x`, `/m`, add-ins, etc.). And the Office URI scheme for Excel (`ms-excel:ofe|u|…`) only takes a document URL—no cell/range arguments. ([Microsoft Support][1])

Where you *do* have a targetable cell/range is **Excel for the web** (OneDrive/SharePoint): the viewer supports URL parameters like `ActiveCell='Sheet1'!B4` (and `Item=` for named objects) when embedding or linking via the WOPI/Embed URL. Example parameters are shown in Microsoft’s embed docs. ([Microsoft Support][2])

### What to do for `srcuri`

To make “Excel + range” work reliably everywhere:

1. **Web path (works today on O365 web):**

    * Generate:

      ```
      https://.../WopiFrame.aspx?...&action=embedview&ActiveCell='Sheet1'!B4
      ```
    * Your bridge can detect `xlsx` + `#'Sheet'!A1` (or `L/C/R`) and 302 to the Excel Web URL above. Users land on the right cell in the browser. ([Microsoft Support][2])

2. **Desktop path (needs a tiny helper):**
   Because neither `excel.exe` nor `ms-excel:` takes a range, have your local opener do:

    * Launch Excel with the file (plain `excel.exe "C:\path\file.xlsx"`). ([Microsoft Support][1])
    * Then **navigate via automation**:

        * **Windows:** COM: `Excel.Application.Workbooks.Open(...)` then `Application.Goto "Sheet1!B4"` (or select a named range).
        * **macOS:** AppleScript: open the file, then `activate object range "B4" of worksheet "Sheet1"`.
    * This makes `srcuri` links like:

      ```
      https://srcuri.com/o/budget.xlsx#'Sheet1'!B4
      ```

      behave as expected on desktop by letting your helper select the target after the file opens.

3. **Nice-to-haves for authors**

    * Allow `#'Sheet'!A1` or `@Sheet1!A1` in the path (no query gymnastics).
    * Support **named ranges** too: `#TotalsTable` → your helper calls `Application.Goto "TotalsTable"`.

### Practical implications

* **CLI / ms-excel URIs:** file only; no cell/range targeting. (So desktop deep-links require your helper.) ([Microsoft Support][1])
* **Excel for the web:** you *can* jump to a cell/range with `ActiveCell=...` (and similar viewer params). ([Microsoft Support][2])
* **External “file://…#Sheet!Cell” fragments:** generally not honored by desktop Excel when launched externally (community & MS Q&A threads confirm there isn’t a supported external URL to a specific cell). ([superuser.com][3])

If you want, I can sketch the tiny Windows COM + macOS AppleScript that your `srcuri` opener would run to make those range jumps seamless.

[1]: https://support.microsoft.com/en-us/office/command-line-switches-for-microsoft-office-products-079164cd-4ef5-4178-b235-441737deb3a6 "Command-line switches for Microsoft Office products - Microsoft Support"
[2]: https://support.microsoft.com/en-us/office/embed-your-excel-workbook-on-your-web-page-or-blog-from-sharepoint-or-onedrive-for-business-7af74ce6-e8a0-48ac-ba3b-a1dd627b7773?utm_source=chatgpt.com "Embed your Excel workbook on your web page or blog ..."
[3]: https://superuser.com/questions/205728/specify-a-specific-worksheet-to-open-for-a-microsoft-excel-workbook-using-a-hype?utm_source=chatgpt.com "Specify a specific worksheet to open for a Microsoft Excel ..."
---
**Google Sheets is actually friendlier here than desktop Excel 🤷‍♂️ — it already has a URL format for “open this sheet and select this range.”

Here’s the basic shape you see in the address bar:

```text
https://docs.google.com/spreadsheets/d/SPREADSHEET_ID/edit#gid=SHEET_TAB_ID
```

To jump to a cell or range, you tack on `&range=` in the fragment:

```text
https://docs.google.com/spreadsheets/d/SPREADSHEET_ID/edit#gid=0&range=B12
https://docs.google.com/spreadsheets/d/SPREADSHEET_ID/edit#gid=532849101&range=B12:E45
```

Key bits:

* **`SPREADSHEET_ID`** = the big long ID in the middle.
* **`gid`** = which tab (sheet) to show.
* **`range=`** = A1-style range to select/highlight.

So Sheets already supports “file + tab + range” in a single URL — no helper needed, no COM, no AppleScript.

---

## How to map that to your `srcuri`

You’ve got two directions to think about:

### 1. User already has a Google Sheets URL

Then you can just treat that as the target. Your `srcuri` link can literally carry it:

```text
srcuri://open?kind=gsheet&url=https://docs.google.com/spreadsheets/d/.../edit#gid=0&range=B12
```

Your desktop opener could:

* open it in the browser (easy), or
* try to hand it off to a local Sheets-capable app (less common).

### 2. User only has “sheet + range” (no full URL)

You can support a short, human version and expand it server-side:

```text
https://srcuri.com/o/budget.gsheet#B12:E45
```

Your server/opener would need to know (or look up) the real Google Sheets URL for `budget.gsheet` (maybe user/workspace mapping), then 302 to the real Sheets URL:

```text
https://docs.google.com/spreadsheets/d/REAL_ID/edit#gid=0&range=B12:E45
```

So you can keep the human-friendly, “I just want cell B12” syntax, and still land people in the real Sheet.

---

## Things to keep in mind

* It’s all in the **fragment** (`#...`), so you don’t need to mess with query-parameter encoding for ranges.
* Sheets is perfectly happy with normal A1 ranges.
* If the user is not authorized, Google handles the auth prompt — good for you.
* If you want parity with your editor links, you can treat `#Sheet1!B12` as first-class in your grammar, then translate to `&range=B12&gid=…` behind the scenes.

---

So: unlike Excel desktop, Google Sheets is already in the world you want — “URL == open this doc at this range.” You can just wrap/forward those and stay consistent with your `https://srcuri.com/o/...` story.
