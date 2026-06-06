# NexusCare — Functional Requirements Specification (FRS)

| Item | Details |
|---|---|
| Project | NexusCare |
| Document Type | Functional Requirements Specification (FRS) |
| Version | 2.0 |
| Date | April 2026 |
| Status | Approved for Development |

**Core Features**
1. Shift Marketplace
2. AI Transcriber + Translation

---

## Table of Contents

1. Introduction & Scope
2. User Roles & Permissions
3. Feature 1: Shift Marketplace
   - 3.1 Hospital Shift Creation
   - 3.2 Shift Broadcasting
   - 3.3 Worker Interest & Expression
   - 3.4 Hospital Selection & Assignment
   - 3.5 Worker Acceptance
   - 3.6 Clock In System
   - 3.7 Clock Out & Handover
   - 3.8 Payment Processing
   - 3.9 Ratings & Reviews

---

## 1. Introduction & Scope

### 1.1 Purpose

This document defines all functional requirements for the NexusCare MVP, covering two core features:

| Feature ID | Feature Name | Description |
|---|---|---|
| F1 | Shift Marketplace | Hospital–doctor matching, shift management, clock in/out, payments |

### 1.2 Scope Boundaries

| In Scope | Out of Scope (Post-MVP) |
|---|---|
| Shift creation and broadcasting | Burnout prediction algorithm |
| Worker matching within 5 km | Blockchain audit trails |
| Clock in/out with GPS | <!-- Advanced analytics dashboard --> |
| Basic handover documentation | <!-- Integration with hospital EMRs --> |
| Payment calculation and payout | <!-- AI transcription for 5 languages --> |
| Recurring shifts | <!-- Offline sync --> |
| Rating system (1–5 stars) | <!-- Translation between languages --> |
| | <!-- Video call infrastructure (use 3rd party) --> |
| | <!-- Insurance integration --> |

### 1.3 User Stories Summary

| ID | User Story |
|---|---|
| US-01 | As a Hospital Admin, I want to create a shift so I can find staff quickly |
| US-02 | As a Hospital Admin, I want to see interested workers ranked so I can select the best match |
| US-03 | As a Health Worker, I want to see shifts near me so I can find work |
| US-04 | As a Health Worker, I want to see full job details before accepting so I know what to expect |
| US-05 | As a Health Worker, I want to clock in/out so I get paid for my time |
| US-06 | As a Health Worker, I want to record patient conversations so I don't have to type notes *(not now)* |
| US-07 | As a Health Worker, I want translation so I can understand patients who speak other languages *(not now)* |
| US-08 | As a Hospital Admin, I want to receive handover documentation so patient care continues |
| US-09 | As a Health Worker, I want to get paid automatically after my shift |
| US-10 | As a Hospital Admin, I want to rate workers so I can track quality |

---

## 2. User Roles & Permissions

### 2.1 Role Definitions

| Role ID | Role Name | Primary Actions |
|---|---|---|
| R1 | Hospital Admin | Create shifts, select workers, view shift history, rate workers, approve payments |
| R2 | Health Worker | Find shifts, accept shifts, clock in/out, record patients, view earnings |
| R3 | System Admin | Manage users, resolve disputes, monitor platform health |

### 2.2 Permission Matrix

| Permission | Hospital Admin | Health Worker | System Admin |
|---|:-:|:-:|:-:|
| Create shift | ✅ | ❌ | ✅ |
| View all hospital shifts | ✅ | ❌ | ✅ |
| View own shifts | ❌ | ✅ | ✅ |
| View interested workers | ✅ | ❌ | ✅ |
| Express interest in shift | ❌ | ✅ | ❌ |
| Accept shift assignment | ❌ | ✅ | ❌ |
| Clock in/out | ❌ | ✅ | ❌ |
| Record patient consultation | ❌ | ✅ | ❌ |
| View own clinical notes | ❌ | ✅ | ✅ |
| View hospital's clinical notes | ✅ | ❌ | ✅ |
| Process payment | ✅ | ❌ | ✅ |
| Rate worker | ✅ | ❌ | ❌ |
| Rate hospital | ❌ | ✅ | ❌ |
| View earnings | ❌ | ✅ | ✅ |
| Manage user accounts | ❌ | ❌ | ✅ |
| Resolve disputes | ❌ | ❌ | ✅ |

---

## 3. Feature 1: Shift Marketplace

### 3.1 Hospital Shift Creation (F1-REQ-01)

**3.1.1 Description**
Hospital administrator creates a new shift request that will be broadcast to eligible health workers.

**3.1.2 Preconditions**
- User must be authenticated as Hospital Admin
- Hospital profile must be verified
- Hospital must have valid payment method on file

**3.1.3 Input Fields**

| Field ID | Field Name | Type | Required | Validation Rules |
|---|---|---|---|---|
| F1-F01 | Role Needed | Dropdown | Yes | Options: Doctor, Nurse, Lab Technician, Pharmacist, Midwife, Other |
| F1-F02 | Specialty | Dropdown | No | Conditional on Role: Emergency, General, Pediatrics, ICU, Surgery, Radiology, etc. |
| F1-F03 | Shift Type | Radio | Yes | Options: In-person, Virtual |
| F1-F04 | Start Date | Date Picker | Yes | Must be today or future date |
| F1-F05 | Start Time | Time Picker | Yes | 15-minute increments |
| F1-F06 | Duration | Dropdown | Yes | Options: 2, 4, 6, 8, 12 hours |
| F1-F07 | Urgency | Dropdown | Yes | Options: STAT (within 1 hr), Urgent (within 4 hrs), Normal (today), Scheduled (future) |
| F1-F08 | Hourly Rate | Number | Conditional | Required if not using fixed rate, minimum ₦2,000 |
| F1-F09 | Fixed Rate | Number | Conditional | Required if not using hourly rate, minimum ₦10,000 |
| F1-F10 | Bonus Amount | Number | No | For STAT/Urgent shifts, minimum ₦0 |
| F1-F11 | Job Description | Textarea | Yes | Max 2000 characters, describes role expectations |
| F1-F12 | Tasks List | Array | Yes | List of specific tasks, add/remove items |
| F1-F13 | Equipment Provided | Array | No | What hospital will provide |
| F1-F14 | Requirements | Array | Yes | Qualifications needed, license requirements |
| F1-F15 | Virtual Link | URL | Conditional | Required if Shift Type = Virtual, auto-generated by system |

**3.1.4 Business Rules**

| Rule ID | Rule Description |
|---|---|
| BR-F1-01 | STAT shifts must start within 1 hour of creation |
| BR-F1-02 | Urgent shifts must start within 4 hours of creation |
| BR-F1-03 | Normal shifts must start same day |
| BR-F1-04 | Scheduled shifts can be up to 30 days in future |
| BR-F1-05 | Shift cannot be created if start time is in the past |
| BR-F1-06 | Hospital cannot have more than 10 active (unfilled) shifts at once |
| BR-F1-07 | STAT shifts get automatic +20% bonus (can be overridden) |
| BR-F1-08 | Virtual shifts have no distance restriction for broadcasting |

**3.1.5 Flow**

1. Admin clicks "Create Shift" button
2. System displays shift creation form
3. Admin fills required fields
4. Admin clicks "Preview Shift"
5. System shows preview of how shift will appear to workers
6. Admin clicks "Broadcast Shift"
7. System validates all fields
8. System creates shift record with `status = 'open'`
9. System triggers broadcast to eligible workers
10. System confirms "Shift broadcasted successfully"

**3.1.6 Error Messages**

| Error Condition | Message |
|---|---|
| Start time in past | "Start time cannot be in the past" |
| Invalid rate | "Rate must be at least ₦2,000 per hour or ₦10,000 fixed" |
| Missing required field | "Please fill in all required fields" |
| Too many active shifts | "You have 10 active shifts. Complete or cancel some before creating more" |

---

### 3.2 Shift Broadcasting (F1-REQ-02)

**3.2.1 Description**
System automatically notifies eligible health workers about new shifts within their radius.

**3.2.2 Preconditions**
- Shift created with `status = 'open'`
- System has list of workers with `availability = true`

**3.2.3 Eligibility Criteria for Workers**

| Criterion | Condition |
|---|---|
| Availability | Worker must have availability toggle = ON |
| Role Match | Worker's role matches shift's role needed |
| Qualifications | Worker has required qualifications (if specified) |
| Distance (In-person) | Worker's current location within 5 km of hospital |
| Distance (Virtual) | No distance restriction |
| Not on shift | Worker not currently clocked into another shift |
| Not blocked | Worker not blocked by this hospital |

**3.2.4 Broadcast Rules**

| Rule ID | Rule Description |
|---|---|
| BR-F1-09 | Broadcast sends push notification to eligible workers |
| BR-F1-10 | Workers can opt out of notifications (settings) |
| BR-F1-11 | STAT shifts broadcast every 15 minutes until filled |
| BR-F1-12 | Urgent shifts broadcast every 30 minutes until filled |
| BR-F1-13 | Normal shifts broadcast once |
| BR-F1-14 | Each broadcast creates record in `shift_broadcasts` table |

**3.2.5 Push Notification Content**

| Urgency | Title | Body |
|---|---|---|
| STAT | 🚨 STAT SHIFT: [Role] needed | [Hospital] needs [Role]. Starts in [time]. ₦[rate]/hr |
| Urgent | ⚠️ URGENT: [Role] needed | [Hospital] needs [Role]. Starts in [time]. ₦[rate]/hr |
| Normal | 📍 New shift: [Role] at [Hospital] | [Duration] hour shift. ₦[rate]/hr. [Distance] km away |
| Scheduled | 📅 Scheduled shift: [Role] | [Date] at [Hospital]. Accept now to secure |

---

### 3.3 Worker Interest & Expression (F1-REQ-03)

**3.3.1 Description**
Worker views shift details and expresses interest, allowing hospital to consider them.

**3.3.2 Preconditions**
- Worker authenticated
- Worker `availability = true`
- Shift `status = 'open'`

**3.3.3 Shift Details Display (Worker View)**

| Field | Display Format |
|---|---|
| Hospital Name | Bold, with verified badge |
| Distance | Show in km, with walking/driving estimate |
| Role & Specialty | Bold |
| Start Time | "Today at 2:00 PM" or "Tomorrow at 8:00 AM" |
| Duration | "8 hours" |
| Urgency | Color-coded badge: 🔴 STAT, 🟠 Urgent, 🟢 Normal, 🔵 Scheduled |
| Pay | "₦8,000/hour (₦64,000 total)" or "₦50,000 fixed" |
| Job Description | Full text |
| Tasks | Bulleted list |
| Equipment Provided | Bulleted list |
| Requirements | Bulleted list with checkmarks if worker qualifies |
| Hospital Rating | ⭐ average from previous shifts |

**3.3.4 Worker Actions**

| Action | Description |
|---|---|
| Express Interest | Worker indicates they want to be considered |
| Dismiss | Worker removes shift from their view |
| Save for Later | Worker bookmarks shift |

**3.3.5 Business Rules**

| Rule ID | Rule Description |
|---|---|
| BR-F1-15 | Worker can express interest in unlimited open shifts |
| BR-F1-16 | Expressing interest does not guarantee assignment |
| BR-F1-17 | Worker can withdraw interest at any time before assignment |
| BR-F1-18 | System records timestamp of interest expression |

**3.3.6 Flow**

1. Worker opens app
2. System displays "Shifts Near You" list (sorted by urgency + distance)
3. Worker taps on shift
4. System displays full shift details
5. Worker taps "I'm Interested"
6. System records interest, updates UI to "Interest Expressed"
7. System notifies hospital that new interest received
8. Worker can view all expressed interests in "My Applications" tab

---

### 3.4 Hospital Selection & Assignment (F1-REQ-04)

**3.4.1 Description**
Hospital admin views interested workers ranked by algorithm and selects a worker for assignment.

**3.4.2 Preconditions**
- Shift `status = 'open'`
- At least one worker expressed interest

**3.4.3 Ranking Algorithm — Score (0–100)**

| Component | Weight | Formula |
|---|---|---|
| Distance | 30% | 100 if ≤ 2 km, 70 if ≤ 5 km, 0 if > 5 km |
| Rating | 25% | (worker_rating / 5) × 100 |
| Experience | 20% | min(completed_shifts / 100, 1) × 100 |
| Acceptance Rate | 15% | acceptance_rate (as percentage) |
| Qualification Match | 10% | 100 if all required quals met, else 0 |

**Example Calculation — Worker A:**
- Distance 1.2 km → 100 × 0.30 = 30.0
- Rating 4.9 → (4.9/5) × 100 = 98 × 0.25 = 24.5
- Shifts 45 → (45/100) × 100 = 45 × 0.20 = 9.0
- Acceptance 98% → 98 × 0.15 = 14.7
- Quals match → 100 × 0.10 = 10.0
- **Total Score = 88.2**

**3.4.4 Display Format (Hospital View)**

| Column | Format |
|---|---|
| Rank | 1, 2, 3 … |
| Name | Full name (last name only for privacy until selected) |
| Distance | X.X km |
| Rating | ⭐ X.X (from X shifts) |
| Experience | X shifts completed |
| Acceptance Rate | XX% |
| Quals Match | ✅ or ❌ |
| Score | XX |
| Action | Select button |

**3.4.5 Assignment Flow**

1. Hospital admin opens shift details
2. System displays "Interested Workers" tab with ranked list
3. Admin reviews worker profiles
4. Admin clicks "Select" on preferred worker
5. System shows confirmation dialog: "Send shift offer to [Worker Name]?"
6. Admin confirms
7. System creates `shift_assignment` record with `status = 'offered'`
8. System sends push notification to worker
9. Shift status remains 'open' until worker accepts
10. System starts 30-minute acceptance timer

**3.4.6 Business Rules**

| Rule ID | Rule Description |
|---|---|
| BR-F1-19 | Hospital can only select workers who expressed interest |
| BR-F1-20 | Hospital can select multiple workers sequentially (if first declines) |
| BR-F1-21 | Selected worker has 30 minutes to accept or decline |
| BR-F1-22 | If worker declines, hospital can select next ranked worker |
| BR-F1-23 | If no acceptance after 30 minutes, offer expires and shift returns to 'open' |
| BR-F1-24 | Hospital cannot select same worker twice for same shift |

---

### 3.5 Worker Acceptance (F1-REQ-05)

**3.5.1 Description**
Worker receives shift offer, reviews full job details, and accepts or declines.

**3.5.2 Preconditions**
- Worker authenticated
- Shift has been offered to this worker
- Offer not expired

**3.5.3 Offer Display (Worker View)**

*Header Section:*
```
🏥 LAGOS UNIVERSITY TEACHING HOSPITAL
📋 OFFER: Emergency Doctor
⏰ Today, 2:00 PM - 10:00 PM (8 hours)
💰 ₦8,000/hour (Total: ₦64,000 + ₦5,000 STAT bonus)
⚠️ Offer expires in 28 minutes
```

*Job Details Section (Expandable):*
- Tasks list (what you will do)
- Equipment provided
- Requirements (qualifications needed)
- Hospital contact information

*Data Compliance Agreement (Must Accept):*
- [ ] I agree to comply with Nigeria Data Protection Regulation (NDPR)
- [ ] I will not record or photograph patients without consent
- [ ] I will only use hospital-provided systems for documentation
- [ ] I will complete handover documentation before clocking out
- [ ] I understand that violation may result in account suspension

*Action Buttons:*
- **[ACCEPT SHIFT]** — enabled only after all agreements checked
- **[DECLINE]** — immediate decline
- **[VIEW DETAILS]** — expanded view of job description

**3.5.4 Acceptance Flow**

1. Worker receives push notification: "Shift offer from [Hospital]"
2. Worker opens app → sees offer screen
3. Worker reads job description
4. Worker checks all data compliance checkboxes
5. Worker taps "Accept Shift"
6. System shows confirmation: "Confirm acceptance of shift at [Hospital]?"
7. Worker confirms
8. System updates `shift_assignment.status = 'accepted'`
9. System updates `shift.status = 'assigned'`
10. System cancels offers to other workers
11. System sends confirmation to hospital
12. Shift added to worker's "Upcoming Shifts" list

**3.5.5 Decline Flow**

1. Worker taps "Decline"
2. System shows: "Are you sure? This shift will be offered to another worker."
3. Worker confirms
4. System updates `shift_assignment.status = 'declined'`
5. System notifies hospital: "[Worker] declined the offer"
6. System prompts hospital to select next worker

**3.5.6 Business Rules**

| Rule ID | Rule Description |
|---|---|
| BR-F1-25 | Worker cannot accept if already clocked into another shift |
| BR-F1-26 | Worker cannot accept if shift start time conflicts with another accepted shift |
| BR-F1-27 | Worker cannot accept if they have been blocked by hospital |
| BR-F1-28 | Worker cannot accept if they have outstanding unresolved dispute |
| BR-F1-29 | Acceptance is binding — worker must complete shift or face penalty |

---

### 3.6 Clock In System (F1-REQ-06)

**3.6.1 Description**
Worker clocks in at shift start time with location verification for in-person shifts.

**3.6.2 Preconditions**
- Shift assignment `status = 'accepted'`
- Current time within shift window (start time ± 1 hour)
- Worker authenticated

**3.6.3 Clock In Methods**

| Shift Type | Method | Verification |
|---|---|---|
| In-person | GPS Location | Must be within 100 m of hospital coordinates |
| Virtual | Link Activation | Click virtual consultation link |

**3.6.4 In-Person Clock In Flow**

1. Worker navigates to "My Shifts" → "Upcoming" tab
2. Worker taps "Clock In" on active shift
3. System requests GPS permission (if not already granted)
4. System gets current location
5. System calculates distance to hospital coordinates
6. If distance ≤ 100 m:
   - System shows "✅ Location verified at [Hospital Name]"
   - Worker taps "Confirm Clock In"
   - System records `clock_in_time` and `clock_in_location`
   - Shift status becomes `in_progress`
   - Timer starts
   - Worker sees "Active Shift" screen
7. If distance > 100 m:
   - System shows "❌ You are not at the hospital location"
   - Shows current distance: "You are X km away"
   - Option: "I am at the hospital but GPS is inaccurate"
   - Option: "Take photo of hospital entrance" (fallback)

**3.6.5 Virtual Shift Clock In Flow**

1. Worker navigates to "My Shifts" → "Upcoming" tab
2. Worker taps "Start Virtual Shift"
3. System checks device camera and microphone permissions
4. System activates virtual consultation link
5. System shows: "Virtual shift active. Patients will appear in queue."
6. Worker clicks "Join Consultation"
7. System opens video call interface
8. System records `clock_in_time`
9. Shift status becomes `in_progress`
10. Timer starts

**3.6.6 GPS Fallback (Photo Verification)**

1. Worker clicks "GPS inaccurate" option
2. System requests photo of hospital entrance
3. Worker takes photo
4. System stores photo with timestamp and location
5. System notifies hospital admin: "[Worker] requested manual clock in"
6. Hospital admin can approve or deny
7. If approved, clock in proceeds
8. If denied, worker cannot clock in

**3.6.7 Late Clock In Rules**

| Latency | Rule |
|---|---|
| 0–15 minutes | Allowed, timer starts at actual clock in time |
| 15–30 minutes | Allowed, but pay reduced by 25% for first hour |
| 30–60 minutes | Allowed only with hospital approval |
| > 60 minutes | Cannot clock in, shift marked as missed |

**3.6.8 Business Rules**

| Rule ID | Rule Description |
|---|---|
| BR-F1-30 | Worker can only clock in within 1 hour of shift start |
| BR-F1-31 | Clock in requires internet connection |
| BR-F1-32 | GPS location stored for audit purposes |
| BR-F1-33 | Worker cannot clock in if already clocked into another shift |
| BR-F1-34 | Virtual shift clock in does not require GPS |

---

### 3.7 Clock Out & Handover (F1-REQ-07)

**3.7.1 Description**
Worker ends shift, provides handover documentation, and clocks out.

**3.7.2 Preconditions**
- Shift `status = 'in_progress'`
- Worker authenticated

**3.7.3 Handover Required Fields**

| Field ID | Field Name | Required | Description |
|---|---|---|---|
| F1-H01 | Patients Seen | Yes | Number of patients seen during shift |
| F1-H02 | Critical Patients | Conditional | List of patients requiring immediate follow-up |
| F1-H03 | Pending Tasks | Conditional | Lab results, referrals, medications due |
| F1-H04 | Instructions | Yes | Instructions for incoming staff |
| F1-H05 | Equipment Status | No | Any issues with hospital equipment |

**3.7.4 Auto-Populated Handover Data**

| Data Source | Auto-Populated Fields |
|---|---|
| Clinical Notes (Feature 2) | Number of patients seen, list of patient codes |
| Shift Timer | Total hours worked |
| Pending Tasks | Extracted from uncompleted checklist items |

**3.7.5 Clock Out Flow**

1. Worker taps "End Shift" button
2. System checks: Has handover been completed?
3. If **NO**:
   - System displays handover form
   - Worker fills required fields
   - System auto-populates from clinical notes
   - Worker reviews and confirms
   - System marks `handover_completed = TRUE`
4. If **YES**:
   - System displays handover summary for review
5. System calculates total hours worked
6. System shows payment calculation preview
7. Worker taps "Confirm & Clock Out"
8. System records `clock_out_time` and `clock_out_location`
9. System updates `shift.status = 'completed'`
10. System triggers payment processing
11. Worker receives confirmation: "Shift completed. Payment processing."

**3.7.6 Handover Preview Screen**

```
┌─────────────────────────────────────────────────────────────┐
│                    HANDOVER SUMMARY                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  SHIFT: LUTH Emergency Dept                                 │
│  DATE: April 14, 2026                                       │
│  WORKER: Dr. Abiola                                         │
│                                                              │
│  PATIENTS SEEN: 18                                          │
│                                                              │
│  CRITICAL PATIENTS:                                         │
│  • Patient #2344 - Bed 6 - Respiratory distress             │
│    Needs: Monitor O2 saturation every 30 minutes            │
│  • Patient #2348 - Bed 3 - Chest pain                       │
│    Needs: ECG and troponin when available                   │
│                                                              │
│  PENDING TASKS:                                             │
│  • Lab results for Patients #2345, #2346, #2350             │
│  • Radiology for Patient #2347                              │
│  • Medication: Antibiotics for Patient #2349 at 9PM         │
│                                                              │
│  INSTRUCTIONS FOR INCOMING STAFF:                           │
│  • ICU consult requested for Patient #2344                  │
│  • Discharge summaries ready for Patients #2342, #2343      │
│                                                              │
│  TOTAL HOURS: 8 hours 5 minutes                             │
│  TOTAL PAY: ₦69,000                                         │
│                                                              │
│  [EDIT HANDOVER]              [CONFIRM & CLOCK OUT]          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**3.7.7 Business Rules**

| Rule ID | Rule Description |
|---|---|
| BR-F1-35 | Worker cannot clock out without completing handover |
| BR-F1-36 | Handover is editable for 1 hour after clock out |
| BR-F1-37 | Hospital can request handover revision within 24 hours |
| BR-F1-38 | Payment is held until handover is approved by hospital |
| BR-F1-39 | If no hospital action for 48 hours, handover auto-approved |

---

### 3.8 Payment Processing (F1-REQ-08)

**3.8.1 Description**
Automatic payment calculation and disbursement after shift completion.

**3.8.2 Payment Calculation Formula**

*For Hourly Rate:*
```
Total Pay = (Hours Worked × Hourly Rate) + Bonus - Platform Fee

Where:
- Hours Worked = (clock_out_time - clock_in_time) in hours, rounded to nearest 15 minutes
- Platform Fee = 10% of (Hours Worked × Hourly Rate + Bonus)
```

*For Fixed Rate:*
```
Total Pay = Fixed Rate + Bonus - (Platform Fee × Fixed Rate)
```

**3.8.3 Payment Calculation Example**

```
Shift: 8-hour shift at ₦8,000/hour
Clock in:  1:55 PM
Clock out: 10:00 PM
Hours worked: 8 hours 5 minutes → 8.083 hours

Gross Pay        = 8.083 × ₦8,000 = ₦64,664
STAT Bonus       =                  ₦5,000
Subtotal         =                  ₦69,664
Platform Fee 10% =                  ₦6,966
NET PAY          =                  ₦62,698
```

**3.8.4 Payment Flow**

1. Worker clocks out, `shift.status = 'completed'`
2. System creates payment record with `status = 'pending'`
3. System notifies hospital: "Shift completed. Payment of ₦62,698 pending approval."
4. Hospital admin reviews shift (optional)
5. If hospital approves OR 24 hours pass without dispute:
   - System updates `payment.status = 'processing'`
   - System initiates Paystack transfer to worker's bank account
   - System creates transaction record
6. Paystack processes transfer (1–3 business days)
7. System receives webhook confirmation
8. System updates `payment.status = 'completed'`
9. Worker receives notification: "Payment of ₦62,698 sent to your bank account"
10. System bills hospital (charge saved payment method)

**3.8.5 Payment Dispute Flow**

1. Hospital admin flags shift for dispute within 24 hours
2. System pauses payment
3. Hospital provides reason (e.g., "Worker left early", "Incomplete handover")
4. System notifies worker of dispute
5. Both parties provide evidence
6. System Admin reviews dispute
7. System Admin makes decision:
   - Full payment
   - Partial payment (e.g., 50%)
   - No payment
8. System processes payment accordingly
9. Both parties notified of resolution

**3.8.6 Business Rules**

| Rule ID | Rule Description |
|---|---|
| BR-F1-40 | Payment only processed after handover approval |
| BR-F1-41 | Hospital has 24 hours to dispute payment |
| BR-F1-42 | Platform fee is 10% of gross payment |
| BR-F1-43 | Worker must have valid bank account on file |
| BR-F1-44 | Minimum payout amount: ₦5,000 |
| BR-F1-45 | Payments batched and processed twice daily (10 AM and 4 PM) |

**3.8.7 Worker Earnings View**

```
┌─────────────────────────────────────────────────────────────┐
│                    MY EARNINGS                               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  TOTAL EARNED (ALL TIME): ₦2,450,000                        │
│  THIS MONTH:               ₦385,000                          │
│  PENDING:                  ₦62,698                           │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ TRANSACTION HISTORY                                    │ │
│  ├────────────────────────────────────────────────────────┤ │
│  │ Apr 14, 2026 │ LUTH         │ ₦62,698 │ Completed     │ │
│  │ Apr 12, 2026 │ General Hosp │ ₦45,000 │ Completed     │ │
│  │ Apr 10, 2026 │ Teleclinic   │ ₦10,800 │ Completed     │ │
│  │ Apr 8,  2026 │ LUTH         │ ₦64,000 │ Completed     │ │
│  │ Apr 5,  2026 │ General Hosp │ ₦22,500 │ Completed     │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  [WITHDRAW HISTORY]    [BANK DETAILS]    [DOWNLOAD RECEIPT] │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

### 3.9 Ratings & Reviews (F1-REQ-09)

**3.9.1 Description**
Mutual rating system for hospitals and workers after shift completion.

**3.9.2 Preconditions**
- Shift `status = 'completed'`
- Payment `status = 'completed'` or `'paid'`

**3.9.3 Rating Interface (Hospital Rates Worker)**

```
┌─────────────────────────────────────────────────────────────┐
│                    RATE WORKER                               │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Shift:  April 14, 2026 - Emergency Dept                    │
│  Worker: Dr. Abiola                                         │
│                                                              │
│  How was this worker?                                       │
│                                                              │
│  ★ ★ ★ ★ ★                                                  │
│                                                              │
│  [1 star]  - Poor, would not hire again                     │
│  [2 stars] - Below average                                  │
│  [3 stars] - Satisfactory                                   │
│  [4 stars] - Good, would hire again                         │
│  [5 stars] - Excellent, exceeded expectations               │
│                                                              │
│  Optional comment:                                          │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Dr. Abiola was punctual, professional, and provided    │ │
│  │ excellent handover. Would definitely hire again.       │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  [SKIP]                          [SUBMIT RATING]            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**3.9.4 Rating Interface (Worker Rates Hospital)**

```
┌─────────────────────────────────────────────────────────────┐
│                    RATE HOSPITAL                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Shift:    April 14, 2026 - Emergency Dept                  │
│  Hospital: LUTH                                             │
│                                                              │
│  How was this hospital?                                     │
│                                                              │
│  ★ ★ ★ ★ ★                                                  │
│                                                              │
│  Rate these aspects:                                        │
│  • Staff support:          ★ ★ ★ ★ ☆                        │
│  • Equipment availability: ★ ★ ★ ★ ★                        │
│  • Communication:          ★ ★ ★ ★ ☆                        │
│  • Payment timeliness:     ★ ★ ★ ★ ★                        │
│                                                              │
│  Optional comment:                                          │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Well-organized department. Supportive staff. Would     │ │
│  │ work here again.                                       │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  [SKIP]                          [SUBMIT RATING]            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**3.9.5 Business Rules**

| Rule ID | Rule Description |
|---|---|
| BR-F1-46 | Ratings must be submitted within 7 days of shift completion |
| BR-F1-47 | Both parties can rate independently |
| BR-F1-48 | Ratings are anonymous (worker/hospital names hidden) |
| BR-F1-49 | Average rating displayed on worker/hospital profiles |
| BR-F1-50 | Users can update rating within 48 hours of submission |
| BR-F1-51 | System Admin can remove inappropriate ratings |
