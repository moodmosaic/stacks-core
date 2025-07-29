import fc from "fast-check";
import type { Model, Real } from "./types";
import { txOk } from "@clarigen/test";
import { logCommand, trackCommandRun } from "./utils";

export const MintMultipleSmall = () =>
  fc.record({
    deposits: fc.array(fc.bigInt(100n * 1000000n, 1000n * 1000000n), {
      minLength: 2,
      maxLength: 10,
    }), // 2-10 deposits of 100-1000 STX each.
  }).map((r) => ({
    check: (model: Readonly<Model>) => model.initialized === true,
    run: (model: Model, real: Real) => {
      trackCommandRun(model, "mint-multiple-small");

      let totalDeposited = 0n;
      for (const amount of r.deposits) {
        txOk(
          real.contracts.sip031Indirect.transferStx(
            amount,
            real.contracts.sip031.identifier,
          ),
          real.accounts.wallet_4.address,
        );
        totalDeposited += amount;
      }

      model.balance += totalDeposited;

      logCommand({
        sender: undefined,
        status: "ok",
        action: "mint-multiple-small",
        value: `${r.deposits.length} deposits, total ${totalDeposited}`,
      });
    },
    toString: () => `mint-multiple-small ${r.deposits.length} deposits`,
  }));
