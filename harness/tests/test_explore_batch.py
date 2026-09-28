"""explore_batch parses explorer outcomes and names scenarios like the corpus."""
import explore_batch


def test_case_lines_and_scenario_names():
    line = 'v3sd-swap1-explore00088bc5-profile0: ticks=1620 status=Faulted fault=Some("Simulation(\\"x\\")")'
    m = explore_batch.CASE.match(line)
    assert m and m.group(6) == "Faulted" and m.group(5) == "1620"
    assert explore_batch.scenario_name(m.group(1)) == "corpus_sd_s1_e00088bc5_p0"
    clean = explore_batch.CASE.match("v3-swap0-explore01a74e09-profile2: ticks=6001 status=Finished(Winner(P1)) fault=None")
    assert explore_batch.scenario_name(clean.group(1)) == "corpus_v3_s0_e01a74e09_p2"
