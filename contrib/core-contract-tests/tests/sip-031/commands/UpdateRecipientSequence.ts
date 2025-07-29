import fc from "fast-check";
import type { Model, Real } from "./types";
import { expect } from "vitest";
import { rov, txOk } from "@clarigen/test";
import { getWalletNameByAddress, logCommand, trackCommandRun } from "./utils";

export const UpdateRecipientSequence = (accounts: Real["accounts"]) =>
  fc.record({
    walletIndices: fc.array(fc.integer({ min: 0, max: 9 }), {
      minLength: 2,
      maxLength: 8,
    }),
  }).map((r) => ({
    check: (model: Readonly<Model>) => {
      return model.initialized === true;
    },
    run: (model: Model, real: Real) => {
      trackCommandRun(model, "update-recipient-sequence");

      const wallets = [
        accounts.deployer.address,
        accounts.wallet_1.address,
        accounts.wallet_2.address,
        accounts.wallet_3.address,
        accounts.wallet_4.address,
        accounts.wallet_5.address,
        accounts.wallet_6.address,
        accounts.wallet_7.address,
        accounts.wallet_8.address,
        accounts.wallet_9.address,
      ];

      let currentRecipient = model.recipient;
      let changesCount = 0;

      for (const walletIndex of r.walletIndices) {
        const newRecipient = wallets[walletIndex];
        if (newRecipient !== currentRecipient) {
          txOk(
            real.contracts.sip031.updateRecipient(newRecipient),
            currentRecipient,
          );
          currentRecipient = newRecipient;
          changesCount++;

          expect(rov(real.contracts.sip031.getRecipient())).toBe(
            currentRecipient,
          );
        }
      }

      model.recipient = currentRecipient;

      logCommand({
        sender: getWalletNameByAddress(currentRecipient),
        status: "ok",
        action: "update-recipient-sequence",
        value: `${changesCount} changes, final: ${
          getWalletNameByAddress(currentRecipient)
        }`,
      });
    },
    toString: () => `update-recipient-sequence ${r.walletIndices.length} steps`,
  }));
