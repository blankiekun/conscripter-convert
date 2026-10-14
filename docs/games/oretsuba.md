# 俺たちに翼はない (PS Vita)

The third game, started 2026-10-13. This is the game's committed
record: formats, how its features map to CONscripter, the routes and
progression variables, and its status. The general process is in
`docs/new-game.md`, general lessons in `PLAN.md` §13. `<pkg>` is the
folder holding the game's Vita package and its license.

- **Status:** starting (see "Status" below). Branch `oretsuba`.
- **Scope (2026-10-13):** the **PS Vita version**, the
  whole game **with extras**. Nothing dropped yet.
- **Engine:** not identified yet.

## Source

- `俺たちに翼はない.pkg` (3,267,708,688 bytes): content ID
  `JP0745-PCSG00299_00-ORETSUBA0000VITA`, title ID PCSG00299, APP_VER
  03.010, SDK 3.00, built 2014-02-06 (Prototype's port).
- `work.bin` (512 bytes): the license (RIF) for the same content ID.
- The package is NPDRM-encrypted. Plan: `work.bin` → zRIF, `pkg2zip -x`
  (github.com/lusid1/pkg2zip) to unpack the package, then
  `psvpfsparser` (github.com/motoharu-gosuto/psvpfstools) to remove the
  PFS encryption, into `<pkg>` (for example `app/PCSG00299/` and a
  decrypted copy next to it).

## Walkthrough

Not yet: the source and the route order go here.

## Progression variables

Not yet.

## Status: starting (2026-10-13)

Nothing converts yet.

Next, in order:
1. Decrypt the package (above). The tools are downloaded binaries:
   running them needs approval (blocked on that 2026-10-13).
2. Identify the engine from the decrypted files (eboot strings, archive
   magic numbers), then playbook step 0: walkthrough, this document,
   plan.
