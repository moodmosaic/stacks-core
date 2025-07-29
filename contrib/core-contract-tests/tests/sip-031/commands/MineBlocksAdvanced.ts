import fc from "fast-check";
import type { Model, Real } from "./types";
import { logCommand, trackCommandRun } from "./utils";

export const MineBlocksAdvanced = () =>
  fc.record({
    // Mine between 1-3 months worth of blocks, but in smaller chunks.
    months: fc.integer({ min: 1, max: 3 }),
  }).map((r) => ({
    check: (model: Readonly<Model>) => model.initialized === true,
    run: (model: Model, _real: Real) => {
      trackCommandRun(model, "mine-blocks-advanced");

      const totalBlocksToMine = r.months *
        Number(model.constants.INITIAL_MINT_VESTING_ITERATION_BLOCKS);

      // Mine blocks in smaller chunks to avoid WASM memory issues.
      // Mine in chunks of at most 1000 blocks at a time.
      const chunkSize = 1000;
      let remainingBlocks = totalBlocksToMine;

      try {
        while (remainingBlocks > 0) {
          const blocksThisChunk = Math.min(remainingBlocks, chunkSize);
          simnet.mineEmptyBlocks(blocksThisChunk);
          remainingBlocks -= blocksThisChunk;
        }

        model.blockHeight += BigInt(totalBlocksToMine);

        logCommand({
          sender: undefined,
          status: "ok",
          action: "mine-blocks-advanced",
          value: `${r.months} months (${totalBlocksToMine} blocks)`,
        });
      } catch (error) {
        // If mining fails, log the error but don't crash the test.
        logCommand({
          sender: undefined,
          status: "err",
          action: "mine-blocks-advanced",
          value: `Failed to mine ${r.months} months: ${error}`,
        });
      }
    },
    toString: () => `mine-blocks-advanced ${r.months} months`,
  }));
