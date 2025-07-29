import fc from "fast-check";
import type { Model, Real } from "./types";
import { txOk } from "@clarigen/test";
import { logCommand, trackCommandRun } from "./utils";

export const MintTinyAmount = () =>
  fc.record({
    amount: fc.bigInt(1n, 100n), // 1-100 micro-STX.
  }).map((r) => ({
    check: (model: Readonly<Model>) => model.initialized === true,
    run: (model: Model, real: Real) => {
      trackCommandRun(model, "mint-tiny-amount");

      txOk(
        real.contracts.sip031Indirect.transferStx(
          r.amount,
          real.contracts.sip031.identifier,
        ),
        real.accounts.wallet_4.address,
      );

      model.balance += r.amount;

      logCommand({
        sender: undefined,
        status: "ok",
        action: "mint-tiny-amount",
        value: `amount ${r.amount}`,
      });
    },
    toString: () => `mint-tiny-amount ${r.amount}`,
  }));
