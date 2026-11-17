import assert from "node:assert/strict";
import test from "node:test";
import { branchRows, localBranchNames } from "./branchList.js";

const refs = [
  { name: "develop", sha: "aaa", remote: false },
  { name: "main", sha: "bbb", remote: false },
  { name: "feature/foo", sha: "ccc", remote: false },
  { name: "origin/develop", sha: "aaa", remote: true },
  { name: "origin/main", sha: "ddd", remote: true },
  { name: "origin/feature/foo", sha: "ccc", remote: true },
  { name: "origin/HEAD", sha: "bbb", remote: true },
  { name: "origin/solo", sha: "eee", remote: true },
  { name: "upstream/develop", sha: "aaa", remote: true },
];

test("la local y origin se juntan solo si el commit es el mismo", () => {
  const rows = branchRows(refs);
  const leaves = rows.filter((row) => row.kind === "leaf");
  assert.deepEqual(
    leaves.map((row) => [row.branch, row.synced]),
    [
      ["feature/foo", true],
      ["origin/HEAD", false],
      ["origin/main", false],
      ["origin/solo", false],
      ["upstream/develop", false],
      ["develop", true],
      ["main", false],
    ],
  );
  assert.equal(rows.find((row) => row.path === "origin").count, 3);
  assert.equal(rows.some((row) => row.branch === "origin/develop"), false);
  assert.equal(rows.some((row) => row.path === "origin/feature"), false);
  assert.deepEqual(localBranchNames(refs), ["develop", "main", "feature/foo"]);
});

test("sin commit conocido, o con la carpeta cerrada, origin sigue aparte", () => {
  const plain = branchRows(["develop", "origin/develop"]);
  assert.deepEqual(
    plain.filter((row) => row.kind === "leaf").map((row) => row.branch),
    ["origin/develop", "develop"],
  );
  assert.equal(plain.find((row) => row.branch === "develop").synced, false);
  const shut = branchRows(refs, { origin: true });
  assert.equal(shut.some((row) => row.branch === "origin/main"), false);
  assert.equal(shut.find((row) => row.path === "origin").shut, true);
});

test("si origin solo repite la local, la carpeta desaparece", () => {
  const rows = branchRows([
    { name: "develop", sha: "aaa", remote: false },
    { name: "origin/develop", sha: "aaa", remote: true },
  ]);
  assert.deepEqual(rows.map((row) => row.branch), ["develop"]);
  assert.equal(rows[0].synced, true);
});
