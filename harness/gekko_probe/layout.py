"""Memory layout shared by mkdol.py (the guest program) and probe.py (the host).

Everything lives in MEM1 (0x80000000..0x81800000). The program sits low, the
data regions well above it. Regions are 256 KiB each, enough for 32768 f64
entries; probe.py checks its sweeps fit.
"""

TEXT_ADDR = 0x80003100          # entry point; program is loaded here

CTRL_ADDR = 0x80400000          # control block, written by host / guest
CTRL_START = CTRL_ADDR + 0      # host writes START_MAGIC once inputs are in
CTRL_DONE = CTRL_ADDR + 4       # guest writes DONE_MAGIC when finished
CTRL_N_FRSQRTE = CTRL_ADDR + 8  # counts, written by host before START
CTRL_N_FRES32 = CTRL_ADDR + 12
CTRL_N_FRES64 = CTRL_ADDR + 16
CTRL_ALIVE = CTRL_ADDR + 20     # guest writes ALIVE_MAGIC at entry (boot check)

START_MAGIC = 0xC0DEBEEF
DONE_MAGIC = 0xC0DEF00D
ALIVE_MAGIC = 0xA11FE000

REGION = 0x40000                # 256 KiB per region
FRSQRTE_IN = 0x80410000         # f64[]  input
FRSQRTE_OUT = FRSQRTE_IN + REGION   # f64[]  frsqrte(input)
FRES32_IN = FRSQRTE_OUT + REGION    # f32[]  input (loaded with lfs)
FRES32_OUT = FRES32_IN + REGION     # f32[]  fres(input) stored with stfs
FRES64_IN = FRES32_OUT + REGION     # f64[]  input (loaded with lfd)
FRES64_OUT = FRES64_IN + REGION     # f64[]  fres(input) stored with stfd
END_ADDR = FRES64_OUT + REGION

MAX_F64 = REGION // 8
MAX_F32 = REGION // 4

assert END_ADDR <= 0x81800000
