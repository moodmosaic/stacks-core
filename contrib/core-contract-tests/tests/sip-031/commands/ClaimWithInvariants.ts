import fc from "fast-check";
import type { Model, Real } from "./types";
import {
  calculateClaimable,
  getWalletNameByAddress,
  logCommand,
  trackCommandRun,
} from "./utils";
import { expect } from "vitest";
import { rov, txOk } from "@clarigen/test";

export const ClaimWithInvariants = (accounts: Real["accounts"]) =>
  fc.record({
    sender: fc.constantFrom(
      ...Object.values(accounts).map((x) => x.address),
    ),
  }).map((r) => ({
    check: (model: Readonly<Model>) => {
      const claimable = calculateClaimable(model);
      return model.initialized === true && model.recipient === r.sender &&
        claimable > 0n;
    },
    run: (model: Model, real: Real) => {
      trackCommandRun(model, "claim-with-invariants");

      const expectedClaim = calculateClaimable(model);
      const balanceBefore = rov(
        real.contracts.sip031Indirect.getBalance(
          real.contracts.sip031.identifier,
        ),
      );
      const recipientBalanceBefore = rov(
        real.contracts.sip031Indirect.getBalance(r.sender),
      );

      const receipt = txOk(real.contracts.sip031.claim(), r.sender);

      // Verify claimed amount matches calculation.
      expect(receipt.value).toBe(expectedClaim);

      const balanceAfter = rov(
        real.contracts.sip031Indirect.getBalance(
          real.contracts.sip031.identifier,
        ),
      );
      const recipientBalanceAfter = rov(
        real.contracts.sip031Indirect.getBalance(r.sender),
      );

      // Invariant 1: Contract balance decreased by claimed amount.
      expect(balanceAfter).toBe(balanceBefore - expectedClaim);

      // Invariant 2: Recipient balance increased by claimed amount.
      expect(recipientBalanceAfter).toBe(
        recipientBalanceBefore + expectedClaim,
      );

      // Invariant 3: Total funds conservation.
      const totalFundsBefore = balanceBefore + recipientBalanceBefore;
      const totalFundsAfter = balanceAfter + recipientBalanceAfter;
      expect(totalFundsAfter).toBe(totalFundsBefore);

      // Invariant 4: Remaining balance should match expected unvested amount.
      const monthsElapsed = (model.blockHeight - model.deployBlockHeight) /
        model.constants.INITIAL_MINT_VESTING_ITERATION_BLOCKS;
      const effectiveMonths = monthsElapsed > 24n ? 24n : monthsElapsed;
      const expectedVested = effectiveMonths < 24n
        ? (model.constants.INITIAL_MINT_VESTING_AMOUNT /
          model.constants.INITIAL_MINT_VESTING_ITERATIONS) * effectiveMonths
        : model.constants.INITIAL_MINT_VESTING_AMOUNT;

      const expectedRemaining = effectiveMonths < 24n
        ? model.constants.INITIAL_MINT_VESTING_AMOUNT - expectedVested
        : 0n;

      // Only check if we have exactly the initial amount (no extra deposits).
      if (model.balance === model.constants.INITIAL_MINT_AMOUNT) {
        expect(balanceAfter).toBe(expectedRemaining);
      }

      model.balance -= expectedClaim;
      model.totalClaimed += expectedClaim;

      logCommand({
        sender: getWalletNameByAddress(r.sender),
        status: "ok",
        action: "claim-with-invariants",
        value: `amount ${expectedClaim}`,
      });
    },
    toString: () => `claim-with-invariants`,
  }));
