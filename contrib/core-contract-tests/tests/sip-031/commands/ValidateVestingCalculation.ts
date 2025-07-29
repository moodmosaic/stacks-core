import fc from "fast-check";
import type { Model, Real } from "./types";
import { expect } from "vitest";
import { rov } from "@clarigen/test";
import { Cl } from "@stacks/transactions";
import { logCommand, trackCommandRun } from "./utils";

export const ValidateVestingCalculation = (_accounts: Real["accounts"]) =>
  fc.record({
    futureBlockHeight: fc.bigInt({
      min: 0n,
      max: 1000n * 4383n, // Up to 1000 months worth of blocks
    }),
  }).map((r) => ({
    check: (model: Readonly<Model>) => {
      return model.initialized === true;
    },
    run: (model: Model, real: Real) => {
      trackCommandRun(model, "calc-total-vested");

      const testBlockHeight = model.deployBlockHeight + r.futureBlockHeight;

      // Skip if testing before deployment
      if (testBlockHeight < model.deployBlockHeight) {
        logCommand({
          sender: undefined,
          status: "ok",
          action: "validate vesting calc",
          value: "skipped (before deployment)",
        });
        return;
      }

      // Calculate expected vested amount using our model
      const diff = testBlockHeight - model.deployBlockHeight;
      const monthsElapsed = diff /
        model.constants.INITIAL_MINT_VESTING_ITERATION_BLOCKS;
      const expectedVested = model.constants.INITIAL_MINT_IMMEDIATE_AMOUNT +
        (monthsElapsed < 24n
          ? (model.constants.INITIAL_MINT_VESTING_AMOUNT /
            model.constants.INITIAL_MINT_VESTING_ITERATIONS) * monthsElapsed
          : model.constants.INITIAL_MINT_VESTING_AMOUNT);

      // Call the contract's private calc-total-vested function
      const actual = simnet.callPrivateFn(
        real.contracts.sip031.identifier,
        "calc-total-vested",
        [Cl.uint(testBlockHeight)],
        real.accounts.deployer.address,
      );

      // Invariant: Contract calculation should match our model
      expect(actual.result).toStrictEqual(Cl.uint(expectedVested));

      // Also test calc-claimable-amount
      const claimableActual = rov(
        real.contracts.sip031.calcClaimableAmount(testBlockHeight),
      );

      // For claimable amount, we need to consider the contract's current balance
      const contractBalance = rov(
        real.contracts.sip031Indirect.getBalance(
          real.contracts.sip031.identifier,
        ),
      );
      const reserved = model.constants.INITIAL_MINT_AMOUNT - expectedVested;
      const expectedClaimable = contractBalance > reserved
        ? contractBalance - reserved
        : 0n;

      expect(claimableActual).toBe(expectedClaimable);

      logCommand({
        sender: undefined,
        status: "ok",
        action: "calc-total-vested",
        value: `block ${testBlockHeight}, vested ${expectedVested}`,
      });
    },
    toString: () => `calc-total-vested`,
  }));
