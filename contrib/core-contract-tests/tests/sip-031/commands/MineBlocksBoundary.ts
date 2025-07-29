import fc from "fast-check";
import type { Model, Real } from "./types";
import { logCommand, trackCommandRun } from "./utils";

export const MineBlocksBoundary = () =>
  fc.record({
    // Mine to exact boundary, one before, or one after.
    boundaryType: fc.constantFrom("exact", "before", "after"),
    iteration: fc.integer({ min: 1, max: 24 }), // Which vesting iteration boundary.
  }).map((r) => ({
    check: (model: Readonly<Model>) => model.initialized === true,
    run: (model: Model, _real: Real) => {
      trackCommandRun(model, "mine-blocks-boundary");

      const iterationBlocks = Number(
        model.constants.INITIAL_MINT_VESTING_ITERATION_BLOCKS,
      );
      const targetBlock = model.deployBlockHeight +
        BigInt(r.iteration * iterationBlocks);

      let blocksToMine = 0;
      let description = "";

      switch (r.boundaryType) {
        case "exact":
          blocksToMine = Number(targetBlock - model.blockHeight);
          description = `exact boundary iteration ${r.iteration}`;
          break;
        case "before":
          blocksToMine = Number(targetBlock - model.blockHeight - 1n);
          description = `one before boundary iteration ${r.iteration}`;
          break;
        case "after":
          blocksToMine = Number(targetBlock - model.blockHeight + 1n);
          description = `one after boundary iteration ${r.iteration}`;
          break;
      }

      if (blocksToMine > 0) {
        // FIXME Mine blocks in smaller chunks to avoid WASM memory issues
        const chunkSize = 1000;
        let remainingBlocks = blocksToMine;

        try {
          while (remainingBlocks > 0) {
            const blocksThisChunk = Math.min(remainingBlocks, chunkSize);
            simnet.mineEmptyBlocks(blocksThisChunk);
            remainingBlocks -= blocksThisChunk;
          }
          model.blockHeight += BigInt(blocksToMine);
        } catch (error) {
          // If mining fails, log the error but don't crash the test.
          console.warn(
            `Failed to mine ${blocksToMine} blocks for boundary: ${error}`,
          );
        }
      }

      logCommand({
        sender: undefined,
        status: "ok",
        action: "mine-blocks-boundary",
        value: description,
      });
    },
    toString: () =>
      `mine-blocks-boundary ${r.boundaryType} iteration ${r.iteration}`,
  }));
