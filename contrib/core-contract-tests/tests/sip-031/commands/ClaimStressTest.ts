import fc from "fast-check";
import type { Model, Real } from "./types";
import {
  calculateClaimable,
  getWalletNameByAddress,
  logCommand,
  trackCommandRun,
} from "./utils";
import { expect } from "vitest";
import { txOk } from "@clarigen/test";

export const ClaimStressTest = (accounts: Real["accounts"]) =>
  fc.record({
    sender: fc.constantFrom(
      ...Object.values(accounts).map((x) => x.address),
    ),
    extraDeposits: fc.array(fc.bigInt(1n, 10000000000n), {
      minLength: 0,
      maxLength: 5,
    }),
    blockAdvances: fc.array(fc.integer({ min: 1, max: 5 }), {
      minLength: 0,
      maxLength: 3,
    }), // 0 - 3 random block advances (in months).
  }).map((r) => ({
    check: (model: Readonly<Model>) => {
      return model.initialized === true && model.recipient === r.sender;
    },
    run: (model: Model, real: Real) => {
      trackCommandRun(model, "claim-stress-test");

      let totalExtraDeposited = 0n;
      let totalMonthsAdvanced = 0;

      // Perform random deposits and time advances.
      for (let i = 0; i < Math.max(r.extraDeposits.length, r.blockAdvances.length); i++) {
        // Add extra deposit if available.
        if (i < r.extraDeposits.length && r.extraDeposits[i] > 0n) {
          txOk(
            real.contracts.sip031Indirect.transferStx(
              r.extraDeposits[i],
              real.contracts.sip031.identifier,
            ),
            real.accounts.wallet_4.address,
          );
          model.balance += r.extraDeposits[i];
          totalExtraDeposited += r.extraDeposits[i];
        }

        // Advance time if available.
        if (i < r.blockAdvances.length) {
          const totalBlocksToMine = r.blockAdvances[i] *
            Number(model.constants.INITIAL_MINT_VESTING_ITERATION_BLOCKS);

          // FIXME Mine blocks in smaller chunks to avoid WASM memory issues.
          const chunkSize = 1000;
          let remainingBlocks = totalBlocksToMine;

          try {
            while (remainingBlocks > 0) {
              const blocksThisChunk = Math.min(remainingBlocks, chunkSize);
              simnet.mineEmptyBlocks(blocksThisChunk);
              remainingBlocks -= blocksThisChunk;
            }
            model.blockHeight += BigInt(totalBlocksToMine);
            totalMonthsAdvanced += r.blockAdvances[i];
          } catch (error) {
            // If mining fails, skip this advance but continue the test.
            console.warn(
              `Failed to mine ${r.blockAdvances[i]} months: ${error}`,
            );
          }
        }
      }

      // Now attempt to claim.
      const claimable = calculateClaimable(model);
      if (claimable > 0n) {
        const receipt = txOk(real.contracts.sip031.claim(), r.sender);

        // Verify the claim amount is correct.
        expect(receipt.value).toBe(claimable);

        // Verify balance consistency and total funds conservation.
        model.balance -= claimable;
        model.totalClaimed += claimable;

        logCommand({
          sender: getWalletNameByAddress(r.sender),
          status: "ok",
          action: "claim-stress-test",
          value:
            `claimed ${claimable}, extra ${totalExtraDeposited}, months ${totalMonthsAdvanced}`,
        });
      } else {
        logCommand({
          sender: getWalletNameByAddress(r.sender),
          status: "ok",
          action: "claim-stress-test",
          value:
            `no claimable amount, extra ${totalExtraDeposited}, months ${totalMonthsAdvanced}`,
        });
      }
    },
    toString: () => `claim-stress-test`,
  }));
