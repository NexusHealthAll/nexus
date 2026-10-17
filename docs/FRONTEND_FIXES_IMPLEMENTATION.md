# Frontend Implementation Guide — Latest Backend Fixes

This covers the six fixes and exactly what the **frontend** needs to do for each.
All routes are under the same base URL as the rest of the API and require the
usual `Authorization: Bearer <JWT>` header unless noted.

| # | Feature | Frontend work | New/changed endpoint |
|---|---|---|---|
| 1 | Handover note images | Upload to Cloudinary, send URLs | `POST /shifts/{id}/handover` (new `image_urls`) |
| 2 | 48h handover approval | Display only (automatic) | — |
| 3 | Handover reminder/appeal | Button + call endpoint | `POST /shifts/{id}/handover/appeal` |
| 4 | Platform fee cap | Admin settings field | `GET/PUT /admin/settings` (new `platform_fee_cap_kobo`) |
| 5 | Shift file attachments | Upload to Cloudinary, send URLs | `POST /shifts` (new `attachment_urls`) |
| 6 | ~~10 km geofence for call~~ **withdrawn** | Nothing — joining a call is not location-gated | `POST /shifts/{id}/consult/token` (`lat`/`lng` accepted, ignored) |

---

## 0. Cloudinary direct upload (shared by #1 and #5)

**The backend never receives file bytes.** For any image/file, upload straight to
Cloudinary using a short-lived signature, then send the returned `secure_url`(s)
to the relevant endpoint.

### Step 1 — get a signature
```
GET /api/v1/uploads/signature?kind=handover
Authorization: Bearer <JWT>
```
`kind` picks the Cloudinary folder. Valid values: `handover`, `shift`,
`hospital_logo`, `worker_avatar` (anything else → a default folder).

**Response:**
```json
{
  "cloud_name": "h9jpvcxe",
  "api_key": "967779646155566",
  "timestamp": 1725800000,
  "folder": "nexuscare/handovers",
  "signature": "<hex>",
  "upload_url": "https://api.cloudinary.com/v1_1/h9jpvcxe/image/upload"
}
```

### Step 2 — POST the file directly to Cloudinary
`multipart/form-data` to `upload_url`, echoing the signed fields:
```
file        = <the file>
api_key     = <from response>
timestamp   = <from response>
signature   = <from response>
folder      = <from response>
```
Cloudinary responds with `{ "secure_url": "https://res.cloudinary.com/.../x.jpg", ... }`.

### Step 3 — send the `secure_url`(s) to the backend
Collect the `secure_url` of each upload into an array and pass it to the handover
submit (#1) or shift create (#5) below. **Do not** send the file to our API.

> Signatures are short-lived — fetch one per upload batch, not once at app start.

---

## 1. Handover note images

Workers already submit a handover at clock-out. The submit body now accepts an
optional `image_urls` array (the Cloudinary `secure_url`s from section 0).

```
POST /api/v1/shifts/{shift_id}/handover        (role: health worker)
```
```json
{
  "patients_seen": 5,
  "critical_patients": [],
  "pending_tasks": [],
  "instructions": "Handed over ward B; watch bed 3.",
  "equipment_status": "Monitor in bay 2 faulty",
  "image_urls": [
    "https://res.cloudinary.com/h9jpvcxe/image/upload/v1/nexuscare/handovers/a.jpg",
    "https://res.cloudinary.com/h9jpvcxe/image/upload/v1/nexuscare/handovers/b.jpg"
  ]
}
```
`image_urls` is optional (defaults to `[]`). Re-submitting within the edit window
replaces the previous images.

**Reading a handover** (hospital that owns the shift, a super admin, or the
assigned worker):
```
GET /api/v1/shifts/{shift_id}/handover
```
The response now includes `image_urls`, plus `appeal_raised_at` and `appeal_note`
(see #3). Render `image_urls` as a thumbnail gallery.

---

## 2. 48-hour handover approval — display only

Already automatic on the backend: if the hospital doesn't act, the handover is
auto-approved 48 hours after submission and payout is released. **No API call
needed.** Use the handover fields to show status:

- `hospital_approved_at` — set when approved (manually or auto).
- `auto_approve_after` — the 48h deadline (show a countdown / "auto-approves on …").
- `revision_requested_at` / `revision_notes` — hospital asked for changes.

---

## 3. Handover reminder / appeal

Once a handover has been **awaiting approval for more than a day**, the assigned
worker can nudge the hospital. This records the appeal and emails the hospital.

```
POST /api/v1/shifts/{shift_id}/handover/appeal   (role: health worker)
```
```json
{ "note": "Please review — I need this approved to get paid." }
```
`note` is optional (max 1000 chars). Returns the updated handover with
`appeal_raised_at` and `appeal_note` set.

**When to show the "Remind / Appeal" button:** the handover exists and
- `now - submitted_at > 24h`, **and**
- `hospital_approved_at` is `null`, **and**
- `appeal_raised_at` is `null` (hide/disable it after one appeal).

**Error responses to handle:**
- `409` — too early (<1 day), already approved, or already appealed → show the message.
- `403` — not the assigned worker.
- `404` — no handover submitted yet.

---

## 4. Platform fee cap (admin dashboard only)

The platform settings now include an optional **fee cap** in kobo. It's applied
automatically at payout (`fee = min(gross × fee% , cap)`); the frontend only
needs to expose the field in the admin settings screen.

```
GET /api/v1/admin/settings      (super admin)
PUT /api/v1/admin/settings      (super admin)
```
`GET` now returns `platform_fee_cap_kobo` (a number in kobo, or `null` = no cap)
alongside `platform_fee_percent`. `PUT` accepts either/both:
```json
{ "platform_fee_percent": 12.5, "platform_fee_cap_kobo": 500000 }
```
> Note: sending `platform_fee_cap_kobo: null` in `PUT` keeps the existing cap
> (partial update). Clearing a cap isn't supported yet — flag us if you need it.

No worker/hospital-facing work for this item.

---

## 5. Shift creation file attachments

The shift-create body now accepts an optional `attachment_urls` array (Cloudinary
`secure_url`s from section 0, using `kind=shift`).

```
POST /api/v1/shifts             (role: hospital admin)
```
```json
{
  "role_category": "doctor",
  "role_title": "Emergency Doctor",
  "...": "all existing fields unchanged",
  "attachment_urls": [
    "https://res.cloudinary.com/h9jpvcxe/image/upload/v1/nexuscare/shifts/brief.pdf"
  ]
}
```
`attachment_urls` is optional (defaults to `[]`). `GET /api/v1/shifts/{id}`
returns `attachment_urls` on the shift so you can list/download them on the
shift-detail screen.

---

## 6. Joining a call is not location-gated (geofence withdrawn)

The 10 km geofence this section originally described **has been removed.** A
virtual consultation is attended from wherever the clinician happens to be —
home, a second site, in transit — so distance from the hospital is never a
condition of getting a join token.

```
POST /api/v1/shifts/{shift_id}/consult/token    (role: health worker / hospital admin)
```
```json
{ "device_label": "iPhone 14" }
```

**Behaviour for any clinician joining a virtual shift:**
- No `lat`/`lng` → **200** with the join token.
- Any distance from the hospital → **200** with the join token.
- `lat`/`lng` are still accepted on the request body so existing clients keep
  working, but the backend ignores them. There is no longer a `400
  LocationRequired` or a `403` distance refusal on this endpoint.

The other join gates are unchanged: the shift must be virtual, its status
joinable, the caller the assigned clinician or an admin of the owning hospital,
and the request inside the consultation window (one hour either side).

**Frontend flow:** drop the geolocation prompt on the pre-join screen entirely.
Do not pre-check distance and do not grey out "Join" for being far away.

### Where the radius does apply: physical GPS clock-in

The distance check belongs to the **physical** clock-in alone
(`POST /api/v1/shifts/{id}/clock-in` with `method: "gps"`), and its radius is
**250 m**, not 10 km:

- Default `clock_in_radius_meters` is **250** (migration `20240072`); a hospital
  can set its own between 50 m and 5000 m on its location record.
- Outside it → **409** `"Clock-in location is N metres from the hospital — outside the geofence"`,
  with the photo-plus-coordinates manual-review fallback as the escape hatch.
- `method: "virtual"` (virtual shifts) takes no coordinates and has no radius.

---

## Testing status

All six were implemented and **verified against a local Postgres database**:

- **#1** — submit handover with `image_urls` → `201`; `GET /handover` returns the new fields → `200`.
- **#3** — appeal before 1 day → `409`; after → `200` (records appeal + emails hospital); second appeal → `409`.
- **#4** — unit test for `fee = min(gross × %, cap)` (percent, cap-wins, cap-noop, 0%, `gross == fee + net`) passes.
- **#5** — shift detail returns `attachment_urls`.
- **#6** — withdrawn; now the opposite is asserted: worker ~525 km away → `200`, no location → `200`
  (`distance_from_the_hospital_never_blocks_a_virtual_join` in `tests/video_consult_tests.rs`).
- **#2** — already existed (auto-approve scheduler); no change.

Build is clean. **Not yet exercised against the Neon/production database** — that
happens when the PR is merged and Render redeploys and runs migrations
`20240058`–`20240061`. The Cloudinary direct-upload leg is frontend-side and uses
the pre-existing signature endpoint.
