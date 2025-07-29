import fc from "fast-check";
import { accounts, project } from "../clarigen-types";
import { projectFactory } from "@clarigen/core";
import { rov } from "@clarigen/test";
import { test } from "vitest";

import { Claim } from "./commands/Claim";
import { ClaimErr } from "./commands/ClaimErr";
import { ClaimWithInvariants } from "./commands/ClaimWithInvariants";
import { ClaimStressTest } from "./commands/ClaimStressTest";
import { MineBlocks } from "./commands/MineBlocks";
import { MineBlocksAdvanced } from "./commands/MineBlocksAdvanced";
import { MineBlocksBoundary } from "./commands/MineBlocksBoundary";
import { Mint } from "./commands/Mint";
import { MintInitial } from "./commands/MintInitial";

import { MintTinyAmount } from "./commands/MintTinyAmount";
import { MintMultipleSmall } from "./commands/MintMultipleSmall";
import { Model, Real } from "./commands/types";
import { UpdateRecipient } from "./commands/UpdateRecipient";
import { UpdateRecipientErr } from "./commands/UpdateRecipientErr";
import { UpdateRecipientSequence } from "./commands/UpdateRecipientSequence";
import { ValidateAccessControl } from "./commands/ValidateAccessControl";
import { ValidateVestingCalculation } from "./commands/ValidateVestingCalculation";
import { reportCommandRuns } from "./commands/utils";

const contracts = projectFactory(project, "simnet");

test("SIP-031 Stateful", () => {
  const real: Real = {
    accounts,
    contracts,
  };

  const model: Model = {
    balance: 0n,
    blockHeight: rov(contracts.sip031.getDeployBlockHeight()),
    constants: contracts.sip031.constants,
    deployBlockHeight: rov(contracts.sip031.getDeployBlockHeight()),
    initialized: false,
    recipient: accounts.deployer.address,
    statistics: new Map(),
    totalClaimed: 0n,
  };

  const invariants = [
    Claim(accounts),
    ClaimErr(accounts),
    ClaimWithInvariants(accounts),
    ClaimStressTest(accounts),
    MineBlocks(),
    MineBlocksAdvanced(),
    MineBlocksBoundary(),
    Mint(),
    MintInitial(accounts),
    MintTinyAmount(),
    MintMultipleSmall(),
    UpdateRecipient(accounts),
    UpdateRecipientErr(accounts),
    UpdateRecipientSequence(accounts),
    ValidateAccessControl(accounts),
    ValidateVestingCalculation(accounts),
  ];

  fc.assert(
    fc.property(
      fc.commands(invariants, { size: "+1" }),
      (cmds) => {
        const state = () => ({ model: model, real: real });
        fc.modelRun(state, cmds);
      },
    ),
    { numRuns: 50, verbose: 2 },
  );

  reportCommandRuns(model);
});
