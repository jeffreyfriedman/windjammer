"""Tip ablation arms: each drops exactly one tip group of the default sheet; packs/ablation/ is never stale."""
import os
import unittest

from agentdx import ablate, arms

T = "t003_prime_counts"


class Ablation(unittest.TestCase):
    def test_committed_packs_equal_a_fresh_render(self):
        for name, text in ablate.render().items():
            self.assertEqual(open(os.path.join(ablate.OUT_DIR, name)).read(), text, f"{name} is stale: run agentdx.ablate.write_all()")

    def test_each_arm_drops_exactly_one_bullet(self):
        full = open(arms.PACK_PATH).read().split("\n")
        for arm, (name, _) in ablate.GROUPS.items():
            pack = open(arms.pack_path(arm)).read().split("\n")
            self.assertEqual(len(full) - len(pack), 1, arm)
            self.assertEqual([l for l in full if l not in pack].__len__(), 1, arm)
            self.assertTrue(arms.ARMS[arm]["pack_file"].endswith(f"minus_{name}.md"))

    def test_control_is_the_default_sheet_and_all_arms_share_prompt_and_loop(self):
        self.assertEqual(open(arms.pack_path("S")).read(), open(arms.PACK_PATH).read())
        self.assertLessEqual(len(open(arms.PACK_PATH).read().encode()), 2048)
        self.assertEqual(len({arms.prompt(T, a) for a in ["S", "B"] + list(ablate.GROUPS)}), 1)

    def test_only_the_tools_ablation_loses_docs_commands(self):
        for a in ["S"] + list(ablate.GROUPS):
            has = "Bash(wj docs:*)" in arms.tool_policy(a)["allowed"]
            self.assertEqual(has, a != "Sf", a)

    def test_drop_rejects_ambiguous_or_missing_groups(self):
        with self.assertRaises(ValueError):
            ablate.drop("- a\n- a2\n", "- a")
        with self.assertRaises(ValueError):
            ablate.drop("x\n", "- nothing")

    def test_dropped_bullets_are_really_gone_from_each_pack(self):
        for arm, (name, prefix) in ablate.GROUPS.items():
            self.assertFalse([l for l in open(arms.pack_path(arm)).read().split("\n") if l.startswith(prefix)], arm)


if __name__ == "__main__":
    unittest.main()
