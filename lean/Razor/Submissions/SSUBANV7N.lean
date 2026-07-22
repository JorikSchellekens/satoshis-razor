import Std.Tactic.BVDecide

/-!
Anvil challenge ANV-007: CRC-64/XZ of the 8 bytes of a u64.

The executable specification is `crcBitwise`: xor the whole little-endian
word into the all-ones register, take 64 single-bit steps of the
reflected ECMA-182 polynomial, complement the result
(anvil/impls/crc64-bitwise). The word-at-once form is the textbook
byte-at-a-time loop specialized to an exactly-8-byte message; the Rust
crate's tests check that equivalence and the catalog check value.

The contender model is `crcNibble`: sixteen 4-bit steps through a 16-entry
table of precomputed remainders (anvil/impls/crc64-nibble). The table
constants appear verbatim below, so `crc_nibble_refines` - checked by
`bv_decide` over all 2^64 inputs - is also a kernel-checked audit of every
table entry: one wrong constant and the theorem is false.
-/

namespace Razor.Anvil

set_option maxRecDepth 16384
set_option maxHeartbeats 2000000

/-- The reflected ECMA-182 polynomial (CRC-64/XZ). -/
def crcPoly : BitVec 64 := 0xC96C5795D7870F42#64

/-- One reflected CRC step: shift down, xor the polynomial if a one fell
off the bottom. -/
def crcStep (crc : BitVec 64) : BitVec 64 :=
  bif crc &&& 1#64 == 1#64 then (crc >>> 1) ^^^ crcPoly else crc >>> 1

/-- `k` CRC bit-steps. -/
def crcSteps : Nat → BitVec 64 → BitVec 64
  | 0, c => c
  | k + 1, c => crcSteps k (crcStep c)

/-- Executable spec: CRC-64/XZ of the 8 little-endian bytes of `x`
(model of crc64-bitwise). Init all ones, 64 bit-steps, final complement -
`~~~x` is the all-ones init xored with the whole message word. -/
def crcBitwise (x : BitVec 64) : BitVec 64 :=
  ~~~ (crcSteps 64 (~~~ x))

/-- One nibble step: shift down four, xor the precomputed remainder of the
four bits that fell off. The sixteen constants are the crc64-nibble
table, verbatim. -/
def crcNibStep (crc : BitVec 64) : BitVec 64 :=
  let n := crc &&& 0xF#64
  let t :=
    bif n == 0x0#64 then 0x0000000000000000#64 else
    bif n == 0x1#64 then 0x7D9BA13851336649#64 else
    bif n == 0x2#64 then 0xFB374270A266CC92#64 else
    bif n == 0x3#64 then 0x86ACE348F355AADB#64 else
    bif n == 0x4#64 then 0x64B62BCAEBC387A1#64 else
    bif n == 0x5#64 then 0x192D8AF2BAF0E1E8#64 else
    bif n == 0x6#64 then 0x9F8169BA49A54B33#64 else
    bif n == 0x7#64 then 0xE21AC88218962D7A#64 else
    bif n == 0x8#64 then 0xC96C5795D7870F42#64 else
    bif n == 0x9#64 then 0xB4F7F6AD86B4690B#64 else
    bif n == 0xA#64 then 0x325B15E575E1C3D0#64 else
    bif n == 0xB#64 then 0x4FC0B4DD24D2A599#64 else
    bif n == 0xC#64 then 0xADDA7C5F3C4488E3#64 else
    bif n == 0xD#64 then 0xD041DD676D77EEAA#64 else
    bif n == 0xE#64 then 0x56ED3E2F9E224471#64 else
    0x2B769F17CF112238#64
  (crc >>> 4) ^^^ t

/-- `k` CRC nibble-steps. -/
def crcNibSteps : Nat → BitVec 64 → BitVec 64
  | 0, c => c
  | k + 1, c => crcNibSteps k (crcNibStep c)

/-- Model of crc64-nibble: sixteen table-driven nibble steps. -/
def crcNibble (x : BitVec 64) : BitVec 64 :=
  ~~~ (crcNibSteps 16 (~~~ x))

/-- The SAT core: one nibble step is exactly four bit steps. This checks
all sixteen table entries at once, on a circuit small enough to settle
in seconds. -/
theorem crc_nib_step_is_four_bits (c : BitVec 64) :
    crcNibStep c = crcStep (crcStep (crcStep (crcStep c))) := by
  simp only [crcNibStep, crcStep, crcPoly]
  bv_decide (config := { timeout := 300 })

/-- ANV-007 admission proof for crc64-nibble: the table-driven walk agrees
with the bit-at-a-time spec on every input. Unfold both walks and rewrite
each nibble step to its four bit steps - the whole-input SAT check lives
in `crc_nib_step_is_four_bits`. -/
theorem crc_nibble_refines (x : BitVec 64) : crcNibble x = crcBitwise x := by
  simp (config := { maxSteps := 4000000 }) only
    [crcNibble, crcBitwise, crcSteps, crcNibSteps, crc_nib_step_is_four_bits]

/-! ### The C lane: a byte at a time through a 256-entry table
(anvil/lanes/crc64-byte-c - entered through the external-lane protocol,
written in C; the model and proof obligations are identical to a Rust
lane's). -/

/-- Row 0x0 of the byte table (high nibble of the retired byte). -/
def crcRow0 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x0000000000000000#64 else
    bif n == 1#64 then 0xB32E4CBE03A75F6F#64 else
    bif n == 2#64 then 0xF4843657A840A05B#64 else
    bif n == 3#64 then 0x47AA7AE9ABE7FF34#64 else
    bif n == 4#64 then 0x7BD0C384FF8F5E33#64 else
    bif n == 5#64 then 0xC8FE8F3AFC28015C#64 else
    bif n == 6#64 then 0x8F54F5D357CFFE68#64 else
    bif n == 7#64 then 0x3C7AB96D5468A107#64 else
    bif n == 8#64 then 0xF7A18709FF1EBC66#64 else
    bif n == 9#64 then 0x448FCBB7FCB9E309#64 else
    bif n == 10#64 then 0x0325B15E575E1C3D#64 else
    bif n == 11#64 then 0xB00BFDE054F94352#64 else
    bif n == 12#64 then 0x8C71448D0091E255#64 else
    bif n == 13#64 then 0x3F5F08330336BD3A#64 else
    bif n == 14#64 then 0x78F572DAA8D1420E#64 else
    0xCBDB3E64AB761D61#64

/-- Row 0x1 of the byte table (high nibble of the retired byte). -/
def crcRow1 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x7D9BA13851336649#64 else
    bif n == 1#64 then 0xCEB5ED8652943926#64 else
    bif n == 2#64 then 0x891F976FF973C612#64 else
    bif n == 3#64 then 0x3A31DBD1FAD4997D#64 else
    bif n == 4#64 then 0x064B62BCAEBC387A#64 else
    bif n == 5#64 then 0xB5652E02AD1B6715#64 else
    bif n == 6#64 then 0xF2CF54EB06FC9821#64 else
    bif n == 7#64 then 0x41E11855055BC74E#64 else
    bif n == 8#64 then 0x8A3A2631AE2DDA2F#64 else
    bif n == 9#64 then 0x39146A8FAD8A8540#64 else
    bif n == 10#64 then 0x7EBE1066066D7A74#64 else
    bif n == 11#64 then 0xCD905CD805CA251B#64 else
    bif n == 12#64 then 0xF1EAE5B551A2841C#64 else
    bif n == 13#64 then 0x42C4A90B5205DB73#64 else
    bif n == 14#64 then 0x056ED3E2F9E22447#64 else
    0xB6409F5CFA457B28#64

/-- Row 0x2 of the byte table (high nibble of the retired byte). -/
def crcRow2 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xFB374270A266CC92#64 else
    bif n == 1#64 then 0x48190ECEA1C193FD#64 else
    bif n == 2#64 then 0x0FB374270A266CC9#64 else
    bif n == 3#64 then 0xBC9D3899098133A6#64 else
    bif n == 4#64 then 0x80E781F45DE992A1#64 else
    bif n == 5#64 then 0x33C9CD4A5E4ECDCE#64 else
    bif n == 6#64 then 0x7463B7A3F5A932FA#64 else
    bif n == 7#64 then 0xC74DFB1DF60E6D95#64 else
    bif n == 8#64 then 0x0C96C5795D7870F4#64 else
    bif n == 9#64 then 0xBFB889C75EDF2F9B#64 else
    bif n == 10#64 then 0xF812F32EF538D0AF#64 else
    bif n == 11#64 then 0x4B3CBF90F69F8FC0#64 else
    bif n == 12#64 then 0x774606FDA2F72EC7#64 else
    bif n == 13#64 then 0xC4684A43A15071A8#64 else
    bif n == 14#64 then 0x83C230AA0AB78E9C#64 else
    0x30EC7C140910D1F3#64

/-- Row 0x3 of the byte table (high nibble of the retired byte). -/
def crcRow3 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x86ACE348F355AADB#64 else
    bif n == 1#64 then 0x3582AFF6F0F2F5B4#64 else
    bif n == 2#64 then 0x7228D51F5B150A80#64 else
    bif n == 3#64 then 0xC10699A158B255EF#64 else
    bif n == 4#64 then 0xFD7C20CC0CDAF4E8#64 else
    bif n == 5#64 then 0x4E526C720F7DAB87#64 else
    bif n == 6#64 then 0x09F8169BA49A54B3#64 else
    bif n == 7#64 then 0xBAD65A25A73D0BDC#64 else
    bif n == 8#64 then 0x710D64410C4B16BD#64 else
    bif n == 9#64 then 0xC22328FF0FEC49D2#64 else
    bif n == 10#64 then 0x85895216A40BB6E6#64 else
    bif n == 11#64 then 0x36A71EA8A7ACE989#64 else
    bif n == 12#64 then 0x0ADDA7C5F3C4488E#64 else
    bif n == 13#64 then 0xB9F3EB7BF06317E1#64 else
    bif n == 14#64 then 0xFE5991925B84E8D5#64 else
    0x4D77DD2C5823B7BA#64

/-- Row 0x4 of the byte table (high nibble of the retired byte). -/
def crcRow4 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x64B62BCAEBC387A1#64 else
    bif n == 1#64 then 0xD7986774E864D8CE#64 else
    bif n == 2#64 then 0x90321D9D438327FA#64 else
    bif n == 3#64 then 0x231C512340247895#64 else
    bif n == 4#64 then 0x1F66E84E144CD992#64 else
    bif n == 5#64 then 0xAC48A4F017EB86FD#64 else
    bif n == 6#64 then 0xEBE2DE19BC0C79C9#64 else
    bif n == 7#64 then 0x58CC92A7BFAB26A6#64 else
    bif n == 8#64 then 0x9317ACC314DD3BC7#64 else
    bif n == 9#64 then 0x2039E07D177A64A8#64 else
    bif n == 10#64 then 0x67939A94BC9D9B9C#64 else
    bif n == 11#64 then 0xD4BDD62ABF3AC4F3#64 else
    bif n == 12#64 then 0xE8C76F47EB5265F4#64 else
    bif n == 13#64 then 0x5BE923F9E8F53A9B#64 else
    bif n == 14#64 then 0x1C4359104312C5AF#64 else
    0xAF6D15AE40B59AC0#64

/-- Row 0x5 of the byte table (high nibble of the retired byte). -/
def crcRow5 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x192D8AF2BAF0E1E8#64 else
    bif n == 1#64 then 0xAA03C64CB957BE87#64 else
    bif n == 2#64 then 0xEDA9BCA512B041B3#64 else
    bif n == 3#64 then 0x5E87F01B11171EDC#64 else
    bif n == 4#64 then 0x62FD4976457FBFDB#64 else
    bif n == 5#64 then 0xD1D305C846D8E0B4#64 else
    bif n == 6#64 then 0x96797F21ED3F1F80#64 else
    bif n == 7#64 then 0x2557339FEE9840EF#64 else
    bif n == 8#64 then 0xEE8C0DFB45EE5D8E#64 else
    bif n == 9#64 then 0x5DA24145464902E1#64 else
    bif n == 10#64 then 0x1A083BACEDAEFDD5#64 else
    bif n == 11#64 then 0xA9267712EE09A2BA#64 else
    bif n == 12#64 then 0x955CCE7FBA6103BD#64 else
    bif n == 13#64 then 0x267282C1B9C65CD2#64 else
    bif n == 14#64 then 0x61D8F8281221A3E6#64 else
    0xD2F6B4961186FC89#64

/-- Row 0x6 of the byte table (high nibble of the retired byte). -/
def crcRow6 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x9F8169BA49A54B33#64 else
    bif n == 1#64 then 0x2CAF25044A02145C#64 else
    bif n == 2#64 then 0x6B055FEDE1E5EB68#64 else
    bif n == 3#64 then 0xD82B1353E242B407#64 else
    bif n == 4#64 then 0xE451AA3EB62A1500#64 else
    bif n == 5#64 then 0x577FE680B58D4A6F#64 else
    bif n == 6#64 then 0x10D59C691E6AB55B#64 else
    bif n == 7#64 then 0xA3FBD0D71DCDEA34#64 else
    bif n == 8#64 then 0x6820EEB3B6BBF755#64 else
    bif n == 9#64 then 0xDB0EA20DB51CA83A#64 else
    bif n == 10#64 then 0x9CA4D8E41EFB570E#64 else
    bif n == 11#64 then 0x2F8A945A1D5C0861#64 else
    bif n == 12#64 then 0x13F02D374934A966#64 else
    bif n == 13#64 then 0xA0DE61894A93F609#64 else
    bif n == 14#64 then 0xE7741B60E174093D#64 else
    0x545A57DEE2D35652#64

/-- Row 0x7 of the byte table (high nibble of the retired byte). -/
def crcRow7 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xE21AC88218962D7A#64 else
    bif n == 1#64 then 0x5134843C1B317215#64 else
    bif n == 2#64 then 0x169EFED5B0D68D21#64 else
    bif n == 3#64 then 0xA5B0B26BB371D24E#64 else
    bif n == 4#64 then 0x99CA0B06E7197349#64 else
    bif n == 5#64 then 0x2AE447B8E4BE2C26#64 else
    bif n == 6#64 then 0x6D4E3D514F59D312#64 else
    bif n == 7#64 then 0xDE6071EF4CFE8C7D#64 else
    bif n == 8#64 then 0x15BB4F8BE788911C#64 else
    bif n == 9#64 then 0xA6950335E42FCE73#64 else
    bif n == 10#64 then 0xE13F79DC4FC83147#64 else
    bif n == 11#64 then 0x521135624C6F6E28#64 else
    bif n == 12#64 then 0x6E6B8C0F1807CF2F#64 else
    bif n == 13#64 then 0xDD45C0B11BA09040#64 else
    bif n == 14#64 then 0x9AEFBA58B0476F74#64 else
    0x29C1F6E6B3E0301B#64

/-- Row 0x8 of the byte table (high nibble of the retired byte). -/
def crcRow8 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xC96C5795D7870F42#64 else
    bif n == 1#64 then 0x7A421B2BD420502D#64 else
    bif n == 2#64 then 0x3DE861C27FC7AF19#64 else
    bif n == 3#64 then 0x8EC62D7C7C60F076#64 else
    bif n == 4#64 then 0xB2BC941128085171#64 else
    bif n == 5#64 then 0x0192D8AF2BAF0E1E#64 else
    bif n == 6#64 then 0x4638A2468048F12A#64 else
    bif n == 7#64 then 0xF516EEF883EFAE45#64 else
    bif n == 8#64 then 0x3ECDD09C2899B324#64 else
    bif n == 9#64 then 0x8DE39C222B3EEC4B#64 else
    bif n == 10#64 then 0xCA49E6CB80D9137F#64 else
    bif n == 11#64 then 0x7967AA75837E4C10#64 else
    bif n == 12#64 then 0x451D1318D716ED17#64 else
    bif n == 13#64 then 0xF6335FA6D4B1B278#64 else
    bif n == 14#64 then 0xB199254F7F564D4C#64 else
    0x02B769F17CF11223#64

/-- Row 0x9 of the byte table (high nibble of the retired byte). -/
def crcRow9 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xB4F7F6AD86B4690B#64 else
    bif n == 1#64 then 0x07D9BA1385133664#64 else
    bif n == 2#64 then 0x4073C0FA2EF4C950#64 else
    bif n == 3#64 then 0xF35D8C442D53963F#64 else
    bif n == 4#64 then 0xCF273529793B3738#64 else
    bif n == 5#64 then 0x7C0979977A9C6857#64 else
    bif n == 6#64 then 0x3BA3037ED17B9763#64 else
    bif n == 7#64 then 0x888D4FC0D2DCC80C#64 else
    bif n == 8#64 then 0x435671A479AAD56D#64 else
    bif n == 9#64 then 0xF0783D1A7A0D8A02#64 else
    bif n == 10#64 then 0xB7D247F3D1EA7536#64 else
    bif n == 11#64 then 0x04FC0B4DD24D2A59#64 else
    bif n == 12#64 then 0x3886B22086258B5E#64 else
    bif n == 13#64 then 0x8BA8FE9E8582D431#64 else
    bif n == 14#64 then 0xCC0284772E652B05#64 else
    0x7F2CC8C92DC2746A#64

/-- Row 0xa of the byte table (high nibble of the retired byte). -/
def crcRowA (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x325B15E575E1C3D0#64 else
    bif n == 1#64 then 0x8175595B76469CBF#64 else
    bif n == 2#64 then 0xC6DF23B2DDA1638B#64 else
    bif n == 3#64 then 0x75F16F0CDE063CE4#64 else
    bif n == 4#64 then 0x498BD6618A6E9DE3#64 else
    bif n == 5#64 then 0xFAA59ADF89C9C28C#64 else
    bif n == 6#64 then 0xBD0FE036222E3DB8#64 else
    bif n == 7#64 then 0x0E21AC88218962D7#64 else
    bif n == 8#64 then 0xC5FA92EC8AFF7FB6#64 else
    bif n == 9#64 then 0x76D4DE52895820D9#64 else
    bif n == 10#64 then 0x317EA4BB22BFDFED#64 else
    bif n == 11#64 then 0x8250E80521188082#64 else
    bif n == 12#64 then 0xBE2A516875702185#64 else
    bif n == 13#64 then 0x0D041DD676D77EEA#64 else
    bif n == 14#64 then 0x4AAE673FDD3081DE#64 else
    0xF9802B81DE97DEB1#64

/-- Row 0xb of the byte table (high nibble of the retired byte). -/
def crcRowB (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x4FC0B4DD24D2A599#64 else
    bif n == 1#64 then 0xFCEEF8632775FAF6#64 else
    bif n == 2#64 then 0xBB44828A8C9205C2#64 else
    bif n == 3#64 then 0x086ACE348F355AAD#64 else
    bif n == 4#64 then 0x34107759DB5DFBAA#64 else
    bif n == 5#64 then 0x873E3BE7D8FAA4C5#64 else
    bif n == 6#64 then 0xC094410E731D5BF1#64 else
    bif n == 7#64 then 0x73BA0DB070BA049E#64 else
    bif n == 8#64 then 0xB86133D4DBCC19FF#64 else
    bif n == 9#64 then 0x0B4F7F6AD86B4690#64 else
    bif n == 10#64 then 0x4CE50583738CB9A4#64 else
    bif n == 11#64 then 0xFFCB493D702BE6CB#64 else
    bif n == 12#64 then 0xC3B1F050244347CC#64 else
    bif n == 13#64 then 0x709FBCEE27E418A3#64 else
    bif n == 14#64 then 0x3735C6078C03E797#64 else
    0x841B8AB98FA4B8F8#64

/-- Row 0xc of the byte table (high nibble of the retired byte). -/
def crcRowC (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xADDA7C5F3C4488E3#64 else
    bif n == 1#64 then 0x1EF430E13FE3D78C#64 else
    bif n == 2#64 then 0x595E4A08940428B8#64 else
    bif n == 3#64 then 0xEA7006B697A377D7#64 else
    bif n == 4#64 then 0xD60ABFDBC3CBD6D0#64 else
    bif n == 5#64 then 0x6524F365C06C89BF#64 else
    bif n == 6#64 then 0x228E898C6B8B768B#64 else
    bif n == 7#64 then 0x91A0C532682C29E4#64 else
    bif n == 8#64 then 0x5A7BFB56C35A3485#64 else
    bif n == 9#64 then 0xE955B7E8C0FD6BEA#64 else
    bif n == 10#64 then 0xAEFFCD016B1A94DE#64 else
    bif n == 11#64 then 0x1DD181BF68BDCBB1#64 else
    bif n == 12#64 then 0x21AB38D23CD56AB6#64 else
    bif n == 13#64 then 0x9285746C3F7235D9#64 else
    bif n == 14#64 then 0xD52F0E859495CAED#64 else
    0x6601423B97329582#64

/-- Row 0xd of the byte table (high nibble of the retired byte). -/
def crcRowD (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xD041DD676D77EEAA#64 else
    bif n == 1#64 then 0x636F91D96ED0B1C5#64 else
    bif n == 2#64 then 0x24C5EB30C5374EF1#64 else
    bif n == 3#64 then 0x97EBA78EC690119E#64 else
    bif n == 4#64 then 0xAB911EE392F8B099#64 else
    bif n == 5#64 then 0x18BF525D915FEFF6#64 else
    bif n == 6#64 then 0x5F1528B43AB810C2#64 else
    bif n == 7#64 then 0xEC3B640A391F4FAD#64 else
    bif n == 8#64 then 0x27E05A6E926952CC#64 else
    bif n == 9#64 then 0x94CE16D091CE0DA3#64 else
    bif n == 10#64 then 0xD3646C393A29F297#64 else
    bif n == 11#64 then 0x604A2087398EADF8#64 else
    bif n == 12#64 then 0x5C3099EA6DE60CFF#64 else
    bif n == 13#64 then 0xEF1ED5546E415390#64 else
    bif n == 14#64 then 0xA8B4AFBDC5A6ACA4#64 else
    0x1B9AE303C601F3CB#64

/-- Row 0xe of the byte table (high nibble of the retired byte). -/
def crcRowE (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x56ED3E2F9E224471#64 else
    bif n == 1#64 then 0xE5C372919D851B1E#64 else
    bif n == 2#64 then 0xA26908783662E42A#64 else
    bif n == 3#64 then 0x114744C635C5BB45#64 else
    bif n == 4#64 then 0x2D3DFDAB61AD1A42#64 else
    bif n == 5#64 then 0x9E13B115620A452D#64 else
    bif n == 6#64 then 0xD9B9CBFCC9EDBA19#64 else
    bif n == 7#64 then 0x6A978742CA4AE576#64 else
    bif n == 8#64 then 0xA14CB926613CF817#64 else
    bif n == 9#64 then 0x1262F598629BA778#64 else
    bif n == 10#64 then 0x55C88F71C97C584C#64 else
    bif n == 11#64 then 0xE6E6C3CFCADB0723#64 else
    bif n == 12#64 then 0xDA9C7AA29EB3A624#64 else
    bif n == 13#64 then 0x69B2361C9D14F94B#64 else
    bif n == 14#64 then 0x2E184CF536F3067F#64 else
    0x9D36004B35545910#64

/-- Row 0xf of the byte table (high nibble of the retired byte). -/
def crcRowF (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x2B769F17CF112238#64 else
    bif n == 1#64 then 0x9858D3A9CCB67D57#64 else
    bif n == 2#64 then 0xDFF2A94067518263#64 else
    bif n == 3#64 then 0x6CDCE5FE64F6DD0C#64 else
    bif n == 4#64 then 0x50A65C93309E7C0B#64 else
    bif n == 5#64 then 0xE388102D33392364#64 else
    bif n == 6#64 then 0xA4226AC498DEDC50#64 else
    bif n == 7#64 then 0x170C267A9B79833F#64 else
    bif n == 8#64 then 0xDCD7181E300F9E5E#64 else
    bif n == 9#64 then 0x6FF954A033A8C131#64 else
    bif n == 10#64 then 0x28532E49984F3E05#64 else
    bif n == 11#64 then 0x9B7D62F79BE8616A#64 else
    bif n == 12#64 then 0xA707DB9ACF80C06D#64 else
    bif n == 13#64 then 0x14299724CC279F02#64 else
    bif n == 14#64 then 0x5383EDCD67C06036#64 else
    0xE0ADA17364673F59#64

/-- The 256-entry byte table as a two-level nibble select over the byte
about to be retired. -/
def crcByteEntry (b : BitVec 64) : BitVec 64 :=
  let hi := (b >>> 4) &&& 0xF#64
  let lo := b &&& 0xF#64
  bif hi == 0#64 then crcRow0 lo else
  bif hi == 1#64 then crcRow1 lo else
  bif hi == 2#64 then crcRow2 lo else
  bif hi == 3#64 then crcRow3 lo else
  bif hi == 4#64 then crcRow4 lo else
  bif hi == 5#64 then crcRow5 lo else
  bif hi == 6#64 then crcRow6 lo else
  bif hi == 7#64 then crcRow7 lo else
  bif hi == 8#64 then crcRow8 lo else
  bif hi == 9#64 then crcRow9 lo else
  bif hi == 10#64 then crcRowA lo else
  bif hi == 11#64 then crcRowB lo else
  bif hi == 12#64 then crcRowC lo else
  bif hi == 13#64 then crcRowD lo else
  bif hi == 14#64 then crcRowE lo else
  crcRowF lo

/-- One byte step: shift down eight, xor the precomputed remainder of the
byte that fell off. -/
def crcByteStep (crc : BitVec 64) : BitVec 64 :=
  (crc >>> 8) ^^^ crcByteEntry (crc &&& 0xFF#64)

/-- `k` CRC byte-steps. -/
def crcByteSteps : Nat → BitVec 64 → BitVec 64
  | 0, c => c
  | k + 1, c => crcByteSteps k (crcByteStep c)

/-- Model of crc64-byte-c: eight table-driven byte steps. -/
def crcByte (x : BitVec 64) : BitVec 64 :=
  ~~~ (crcByteSteps 8 (~~~ x))

/-- The SAT core for the byte lane: one byte step is exactly eight bit
steps - which checks all 256 table entries at once. -/
theorem crc_byte_step_is_eight_bits (c : BitVec 64) :
    crcByteStep c =
      crcStep (crcStep (crcStep (crcStep (crcStep (crcStep (crcStep (crcStep c))))))) := by
  simp (config := { maxSteps := 4000000 }) only
    [crcByteStep, crcByteEntry, crcRow0, crcRow1, crcRow2, crcRow3, crcRow4, crcRow5, crcRow6, crcRow7, crcRow8, crcRow9, crcRowA, crcRowB, crcRowC, crcRowD, crcRowE, crcRowF, crcStep, crcPoly]
  bv_decide (config := { timeout := 300 })

/-- ANV-007 admission proof for crc64-byte-c: the byte-table walk agrees
with the bit-at-a-time spec on every input. -/
theorem crc_byte_refines (x : BitVec 64) : crcByte x = crcBitwise x := by
  simp (config := { maxSteps := 4000000 }) only
    [crcByte, crcBitwise, crcSteps, crcByteSteps, crc_byte_step_is_eight_bits]

end Razor.Anvil
