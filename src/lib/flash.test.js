import assert from "node:assert/strict";
import test from "node:test";
import { BUSY_DELAY_MS, ERROR_MS, NOTICE_MS, bannerGone, busyOverlay } from "./flash.js";

test("el aviso se va a los 3s y el error un poco después", () => {
  assert.equal(bannerGone(0, NOTICE_MS, NOTICE_MS), true);
  assert.equal(bannerGone(1000, 1000 + NOTICE_MS - 1, NOTICE_MS), false);
  assert.equal(bannerGone(1000, 1000 + ERROR_MS, ERROR_MS), true);
  assert.equal(bannerGone(0, 0, NOTICE_MS), true);
});

test("el mango no sale en un parpadeo: espera un poco y hay trabajo", () => {
  assert.equal(busyOverlay(1, 0), false);
  assert.equal(busyOverlay(1, BUSY_DELAY_MS), true);
  assert.equal(busyOverlay(0, 1000), false);
  assert.equal(busyOverlay(2, BUSY_DELAY_MS + 10), true);
});
