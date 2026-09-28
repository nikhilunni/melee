"""explore_batch parses explorer outcomes and names scenarios like the corpus."""
import explore_batch


def test_case_lines_and_scenario_names():
    line = 'v3sd-swap1-explore00088bc5-profile0: ticks=1620 status=Faulted fault=Some("Simulation(\\"x\\")")'
    m = explore_batch.CASE.match(line)
    assert m and m.group(6) == "Faulted" and m.group(5) == "1620"
    assert explore_batch.scenario_name(m.group(1)) == "corpus_sd_s1_e00088bc5_p0"
    clean = explore_batch.CASE.match("v3-swap0-explore01a74e09-profile2: ticks=6001 status=Finished(Winner(P1)) fault=None")
    assert explore_batch.scenario_name(clean.group(1)) == "corpus_v3_s0_e01a74e09_p2"


def test_other_boundaries_name_scenarios_after_the_boundary():
    line = 'v3-bf_fox_marth4-explore0c0ffee1-profile1: ticks=86 status=Faulted fault=Some("x")'
    m = explore_batch.CASE.match(line)
    assert m.group("tag") == "bf_fox_marth4" and m.group("ticks") == "86"
    assert explore_batch.scenario_name(m.group("case")) == "corpus_v3_bf_fox_marth4_e0c0ffee1_p1"


def test_pick_faults_keeps_the_shortest_cases_of_each_fault_in_log_order():
    lines = [
        'v3-a-explore00000001-profile0: ticks=500 status=Faulted fault=Some("one")',
        'v3-a-explore00000002-profile0: ticks=90 status=Faulted fault=Some("two")',
        'v3-a-explore00000003-profile0: ticks=100 status=Faulted fault=Some("one")',
        'v3-a-explore00000004-profile0: ticks=300 status=Faulted fault=Some("one")',
    ]
    faults = [explore_batch.CASE.match(line) for line in lines]
    picked = explore_batch.pick_faults(faults, 2)
    assert [m.group("seed") for m in picked] == ["00000002", "00000003", "00000004"]
    assert explore_batch.pick_faults(faults, 0) == faults
