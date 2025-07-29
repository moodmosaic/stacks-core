import fc from "fast-check";
import type { Model, Real } from "./types";
import { expect } from "vitest";
import { rov, txErr } from "@clarigen/test";
import { getWalletNameByAddress, logCommand, trackCommandRun } from "./utils";

export const ValidateAccessControl = (accounts: Real["accounts"]) =>
  fc.record({
    unauthorizedSender: fc.constantFrom(
      ...Object.values(accounts).map((x) => x.address),
    ),
  }).map((r) => ({
    check: (model: Readonly<Model>) => {
      // Only run when there's an unauthorized sender (not the current recipient)
      return model.initialized === true &&
        model.recipient !== r.unauthorizedSender;
    },
    run: (model: Model, real: Real) => {
      trackCommandRun(model, "check-access-control");

      const currentRecipient = rov(real.contracts.sip031.getRecipient());
      expect(currentRecipient).toBe(model.recipient);

      // Invariant 1: Unauthorized sender cannot update recipient
      const updateReceipt = txErr(
        real.contracts.sip031.updateRecipient(real.accounts.deployer.address),
        r.unauthorizedSender,
      );
      expect(updateReceipt.value).toBe(model.constants.ERR_NOT_ALLOWED);

      // Invariant 2: Unauthorized sender cannot claim
      const claimReceipt = txErr(
        real.contracts.sip031.claim(),
        r.unauthorizedSender,
      );
      expect(claimReceipt.value).toBe(model.constants.ERR_NOT_ALLOWED);

      // Invariant 3: Recipient should remain unchanged
      const recipientAfter = rov(real.contracts.sip031.getRecipient());
      expect(recipientAfter).toBe(model.recipient);

      logCommand({
        sender: getWalletNameByAddress(r.unauthorizedSender),
        status: "ok",
        action: "check-access-control",
        value: `blocked unauthorized access`,
      });
    },
    toString: () => `check-access-control`,
  }));
