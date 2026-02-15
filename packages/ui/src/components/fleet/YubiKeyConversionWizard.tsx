import { useState, useCallback, useEffect } from "react";
import {
  CheckCircle2,
  XCircle,
  Loader2,
  KeyRound,
  ShieldCheck,
  Upload,
  Settings,
  ChevronRight,
  AlertTriangle,
  Fingerprint,
} from "lucide-react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { Select } from "../ui/Select";
import { useToastStore } from "../../stores/toastStore";
import { formatError } from "@quartermaster/core";
import * as api from "../../services/tauriCommands";
import type { Node } from "@quartermaster/core";
import type { YubiKeyInfo } from "@quartermaster/core";

// ── Types ──────────────────────────────────────────────────────────────

type WizardStep = "prerequisites" | "generate" | "deploy" | "finalize";

interface StepDef {
  id: WizardStep;
  label: string;
  icon: typeof KeyRound;
}

const STEPS: StepDef[] = [
  { id: "prerequisites", label: "Prerequisites", icon: Settings },
  { id: "generate", label: "Generate Key", icon: KeyRound },
  { id: "deploy", label: "Deploy Key", icon: Upload },
  { id: "finalize", label: "Finalize", icon: ShieldCheck },
];

interface YubiKeyConversionWizardProps {
  open: boolean;
  onClose: () => void;
  node: Node;
  onComplete: () => void;
}

// ── Component ──────────────────────────────────────────────────────────

export function YubiKeyConversionWizard({
  open,
  onClose,
  node,
  onComplete,
}: YubiKeyConversionWizardProps) {
  const { addToast } = useToastStore();

  // Step state
  const [currentStep, setCurrentStep] = useState<WizardStep>("prerequisites");

  // Prerequisites state
  const [depsStatus, setDepsStatus] = useState<api.Fido2DependencyStatus | null>(null);
  const [depsLoading, setDepsLoading] = useState(false);
  const [depsInstalling, setDepsInstalling] = useState(false);
  const [remoteVersion, setRemoteVersion] = useState<api.RemoteSshVersionInfo | null>(null);
  const [remoteVersionLoading, setRemoteVersionLoading] = useState(false);
  const [remoteVersionError, setRemoteVersionError] = useState<string | null>(null);

  // Generate state
  const [yubikeys, setYubikeys] = useState<YubiKeyInfo[]>([]);
  const [yukikeysLoading, setYubikeysLoading] = useState(false);
  const [keyName, setKeyName] = useState(`yubikey-${node.name.toLowerCase().replace(/\s+/g, "-")}`);
  const [resident, setResident] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [generatedKey, setGeneratedKey] = useState<api.SshKeyInfo | null>(null);

  // Deploy state
  const [deploying, setDeploying] = useState(false);
  const [deployed, setDeployed] = useState(false);
  const [deployError, setDeployError] = useState<string | null>(null);

  // Finalize state
  const [switching, setSwitching] = useState(false);
  const [switched, setSwitched] = useState(false);
  const [hardenSshd, setHardenSshd] = useState(false);
  const [hardening, setHardening] = useState(false);
  const [hardenResult, setHardenResult] = useState<api.SshdHardenResult | null>(null);

  // Reset all state when dialog opens
  useEffect(() => {
    if (open) {
      setCurrentStep("prerequisites");
      setDepsStatus(null);
      setDepsLoading(false);
      setDepsInstalling(false);
      setRemoteVersion(null);
      setRemoteVersionLoading(false);
      setRemoteVersionError(null);
      setYubikeys([]);
      setYubikeysLoading(false);
      setKeyName(`yubikey-${node.name.toLowerCase().replace(/\s+/g, "-")}`);
      setResident(true);
      setGenerating(false);
      setGeneratedKey(null);
      setDeploying(false);
      setDeployed(false);
      setDeployError(null);
      setSwitching(false);
      setSwitched(false);
      setHardenSshd(false);
      setHardening(false);
      setHardenResult(null);
    }
  }, [open, node.name]);

  // ── Prerequisites checks ───────────────────────────────────────────

  const checkDeps = useCallback(async () => {
    setDepsLoading(true);
    try {
      const status = await api.checkFido2Dependencies();
      setDepsStatus(status);
    } catch (err) {
      addToast({ type: "error", title: "Failed to check dependencies", message: formatError(err) });
    } finally {
      setDepsLoading(false);
    }
  }, [addToast]);

  const installDeps = useCallback(async () => {
    setDepsInstalling(true);
    try {
      const status = await api.installFido2Dependencies();
      setDepsStatus(status);
      if (status.all_satisfied) {
        addToast({ type: "success", title: "FIDO2 dependencies installed" });
      } else {
        addToast({ type: "error", title: "Installation incomplete", message: "Some dependencies could not be installed." });
      }
    } catch (err) {
      addToast({ type: "error", title: "Failed to install dependencies", message: formatError(err) });
    } finally {
      setDepsInstalling(false);
    }
  }, [addToast]);

  const checkRemoteVersion = useCallback(async () => {
    setRemoteVersionLoading(true);
    setRemoteVersionError(null);
    try {
      const version = await api.checkRemoteSshVersion(node.id);
      setRemoteVersion(version);
      if (!version.supports_sk_keys) {
        setRemoteVersionError(
          `Remote OpenSSH ${version.version_string} does not support security keys. Version 8.2+ is required.`,
        );
      }
    } catch (err) {
      setRemoteVersionError(formatError(err));
    } finally {
      setRemoteVersionLoading(false);
    }
  }, [node.id]);

  // Auto-run prerequisites checks when step becomes active
  useEffect(() => {
    if (open && currentStep === "prerequisites") {
      if (!depsStatus && !depsLoading) checkDeps();
      if (!remoteVersion && !remoteVersionLoading && !remoteVersionError) checkRemoteVersion();
    }
  }, [open, currentStep, depsStatus, depsLoading, remoteVersion, remoteVersionLoading, remoteVersionError, checkDeps, checkRemoteVersion]);

  // ── Generate step ──────────────────────────────────────────────────

  const detectYubikeys = useCallback(async () => {
    setYubikeysLoading(true);
    try {
      const keys = await api.detectYubikeys();
      setYubikeys(keys);
      if (keys.length === 0) {
        addToast({ type: "warning", title: "No YubiKey detected", message: "Insert your YubiKey and try again." });
      }
    } catch (err) {
      addToast({ type: "error", title: "Failed to detect YubiKeys", message: formatError(err) });
    } finally {
      setYubikeysLoading(false);
    }
  }, [addToast]);

  // Auto-detect YubiKeys when entering generate step
  useEffect(() => {
    if (open && currentStep === "generate" && yubikeys.length === 0 && !yukikeysLoading) {
      detectYubikeys();
    }
  }, [open, currentStep, yubikeys.length, yukikeysLoading, detectYubikeys]);

  const handleGenerateKey = useCallback(async () => {
    setGenerating(true);
    try {
      const key = await api.generateFido2SshKey(keyName, `${node.name} YubiKey`, resident);
      setGeneratedKey(key);
      addToast({ type: "success", title: "SSH key generated", message: `Key: ${key.name}` });
    } catch (err) {
      addToast({ type: "error", title: "Key generation failed", message: formatError(err) });
    } finally {
      setGenerating(false);
    }
  }, [keyName, node.name, resident, addToast]);

  // ── Deploy step ────────────────────────────────────────────────────

  const handleDeploy = useCallback(async () => {
    if (!generatedKey) return;
    setDeploying(true);
    setDeployError(null);
    try {
      await api.deployKeyToNode(node.id, generatedKey.path);
      setDeployed(true);
      addToast({ type: "success", title: "Key deployed to remote node" });
    } catch (err) {
      setDeployError(formatError(err));
      addToast({ type: "error", title: "Key deployment failed", message: formatError(err) });
    } finally {
      setDeploying(false);
    }
  }, [generatedKey, node.id, addToast]);

  // Auto-deploy when entering deploy step
  useEffect(() => {
    if (open && currentStep === "deploy" && generatedKey && !deployed && !deploying && !deployError) {
      handleDeploy();
    }
  }, [open, currentStep, generatedKey, deployed, deploying, deployError, handleDeploy]);

  // ── Finalize step ──────────────────────────────────────────────────

  const handleSwitchAuth = useCallback(async () => {
    if (!node.ssh_config) return;
    setSwitching(true);
    try {
      const updatedNode: Node = {
        ...node,
        ssh_config: {
          ...node.ssh_config,
          auth_method: { type: "fido2_resident" },
        },
      };
      await api.updateNode(updatedNode);
      setSwitched(true);
      addToast({ type: "success", title: "Authentication updated to YubiKey" });

      // Optional: harden remote sshd
      if (hardenSshd) {
        setHardening(true);
        try {
          const result = await api.hardenRemoteSshd(node.id, {
            disable_password_auth: true,
            disable_challenge_response: true,
            disable_pam: false,
          });
          setHardenResult(result);
          addToast({ type: "success", title: "Remote SSH hardened" });
        } catch (err) {
          addToast({ type: "warning", title: "SSH hardening failed", message: formatError(err) });
        } finally {
          setHardening(false);
        }
      }

      onComplete();
    } catch (err) {
      addToast({ type: "error", title: "Failed to update node", message: formatError(err) });
    } finally {
      setSwitching(false);
    }
  }, [node, hardenSshd, addToast, onComplete]);

  // ── Navigation ─────────────────────────────────────────────────────

  const stepIndex = STEPS.findIndex((s) => s.id === currentStep);

  const canProceed = (): boolean => {
    switch (currentStep) {
      case "prerequisites":
        return (depsStatus?.all_satisfied ?? false) && (remoteVersion?.supports_sk_keys ?? false);
      case "generate":
        return generatedKey !== null;
      case "deploy":
        return deployed;
      case "finalize":
        return switched;
    }
  };

  const goNext = () => {
    if (stepIndex < STEPS.length - 1) {
      setCurrentStep(STEPS[stepIndex + 1].id);
    }
  };

  const goBack = () => {
    if (stepIndex > 0) {
      setCurrentStep(STEPS[stepIndex - 1].id);
    }
  };

  // ── Render: Step indicator ─────────────────────────────────────────

  const renderStepIndicator = () => (
    <div className="flex items-center gap-1 mb-6">
      {STEPS.map((step, i) => {
        const isActive = step.id === currentStep;
        const isCompleted = i < stepIndex;
        const Icon = step.icon;
        return (
          <div key={step.id} className="flex items-center flex-1">
            <div className="flex items-center gap-2 flex-1">
              <div
                className={`flex items-center justify-center w-8 h-8 rounded-full shrink-0 transition-colors ${
                  isCompleted
                    ? "bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400"
                    : isActive
                      ? "bg-warm-100 dark:bg-warm-900/30 text-warm-600 dark:text-warm-400"
                      : "bg-gray-100 dark:bg-gray-800 text-gray-400 dark:text-gray-600"
                }`}
              >
                {isCompleted ? <CheckCircle2 size={16} /> : <Icon size={16} />}
              </div>
              <span
                className={`text-xs font-medium truncate ${
                  isActive
                    ? "text-text-primary-light dark:text-text-primary-dark"
                    : "text-text-secondary-light dark:text-text-secondary-dark"
                }`}
              >
                {step.label}
              </span>
            </div>
            {i < STEPS.length - 1 && (
              <ChevronRight size={14} className="text-gray-300 dark:text-gray-700 shrink-0 mx-1" />
            )}
          </div>
        );
      })}
    </div>
  );

  // ── Render: Prerequisites step ─────────────────────────────────────

  const renderPrerequisites = () => (
    <div className="space-y-4">
      {/* Local FIDO2 dependencies */}
      <div className="p-4 rounded-lg border border-border-light dark:border-border-dark space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Fingerprint size={16} className="text-warm-500" />
            <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
              Local FIDO2 Libraries
            </span>
          </div>
          {depsLoading ? (
            <Loader2 size={16} className="animate-spin text-warm-400" />
          ) : depsStatus?.all_satisfied ? (
            <Badge variant="success">Installed</Badge>
          ) : depsStatus ? (
            <Badge variant="warning">Missing</Badge>
          ) : null}
        </div>

        {depsStatus && !depsStatus.all_satisfied && (
          <div className="space-y-2">
            <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
              Quartermaster needs <code className="px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-800 text-xs">libfido2</code>
              {!depsStatus.ykman_installed && (
                <> and <code className="px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-800 text-xs">ykman</code></>
              )}{" "}
              to communicate with your YubiKey. Click below to install.
            </p>
            <Button
              variant="primary"
              size="sm"
              onClick={installDeps}
              loading={depsInstalling}
            >
              Install Dependencies
            </Button>
            {depsStatus.install_output && (
              <pre className="text-xs p-2 rounded bg-gray-50 dark:bg-gray-900 border border-border-light dark:border-border-dark max-h-32 overflow-y-auto text-text-secondary-light dark:text-text-secondary-dark">
                {depsStatus.install_output}
              </pre>
            )}
          </div>
        )}

        {depsStatus?.all_satisfied && (
          <div className="flex items-center gap-2">
            <CheckCircle2 size={14} className="text-green-500" />
            <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
              libfido2 and ykman are installed
            </span>
          </div>
        )}
      </div>

      {/* Remote SSH version */}
      <div className="p-4 rounded-lg border border-border-light dark:border-border-dark space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <ShieldCheck size={16} className="text-warm-500" />
            <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
              Remote SSH Compatibility
            </span>
          </div>
          {remoteVersionLoading ? (
            <Loader2 size={16} className="animate-spin text-warm-400" />
          ) : remoteVersion?.supports_sk_keys ? (
            <Badge variant="success">Compatible</Badge>
          ) : remoteVersionError ? (
            <Badge variant="danger">Error</Badge>
          ) : remoteVersion ? (
            <Badge variant="danger">Incompatible</Badge>
          ) : null}
        </div>

        {remoteVersion && (
          <div className="flex items-center gap-2">
            {remoteVersion.supports_sk_keys ? (
              <CheckCircle2 size={14} className="text-green-500" />
            ) : (
              <XCircle size={14} className="text-red-500" />
            )}
            <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
              Remote OpenSSH version: {remoteVersion.version_string}
              {remoteVersion.supports_sk_keys ? " (supports security keys)" : " (8.2+ required for security keys)"}
            </span>
          </div>
        )}

        {remoteVersionError && !remoteVersion && (
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <XCircle size={14} className="text-red-500" />
              <span className="text-xs text-red-600 dark:text-red-400">{remoteVersionError}</span>
            </div>
            <Button variant="secondary" size="sm" onClick={checkRemoteVersion}>
              Retry
            </Button>
          </div>
        )}
      </div>
    </div>
  );

  // ── Render: Generate step ──────────────────────────────────────────

  const renderGenerate = () => (
    <div className="space-y-4">
      {/* YubiKey detection */}
      <div className="p-4 rounded-lg border border-border-light dark:border-border-dark space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Fingerprint size={16} className="text-warm-500" />
            <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
              YubiKey Detection
            </span>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={detectYubikeys}
            loading={yukikeysLoading}
          >
            Refresh
          </Button>
        </div>

        {yubikeys.length > 0 ? (
          <div className="space-y-2">
            {yubikeys.map((yk) => (
              <div
                key={yk.serial}
                className="flex items-center gap-3 p-2 rounded-lg bg-green-50 dark:bg-green-900/10 border border-green-200 dark:border-green-800"
              >
                <CheckCircle2 size={14} className="text-green-500 shrink-0" />
                <div className="flex-1 min-w-0">
                  <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
                    {yk.model || "YubiKey"}
                  </span>
                  <span className="text-xs text-text-secondary-light dark:text-text-secondary-dark ml-2">
                    Serial: {yk.serial}
                  </span>
                </div>
                <Badge variant={yk.fido2_supported ? "success" : "danger"}>
                  {yk.fido2_supported ? "FIDO2" : "No FIDO2"}
                </Badge>
              </div>
            ))}
          </div>
        ) : (
          <div className="flex items-center gap-2 p-2 rounded-lg bg-yellow-50 dark:bg-yellow-900/10 border border-yellow-200 dark:border-yellow-800">
            <AlertTriangle size={14} className="text-yellow-500 shrink-0" />
            <span className="text-xs text-yellow-700 dark:text-yellow-400">
              No YubiKey detected. Insert your YubiKey and click Refresh.
            </span>
          </div>
        )}
      </div>

      {/* Key generation form */}
      <div className="p-4 rounded-lg border border-border-light dark:border-border-dark space-y-3">
        <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
          Generate FIDO2 SSH Key
        </span>

        <div className="space-y-3">
          <div className="flex flex-col gap-1.5">
            <label className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
              Key Name
            </label>
            <input
              type="text"
              value={keyName}
              onChange={(e) => setKeyName(e.target.value)}
              disabled={generating || generatedKey !== null}
              className="w-full rounded-lg border border-border-light dark:border-border-dark px-3 py-2 text-sm bg-surface-light dark:bg-surface-dark text-text-primary-light dark:text-text-primary-dark focus:outline-none focus:ring-2 focus:ring-warm-300/50 focus:border-warm-400 disabled:opacity-50"
            />
          </div>

          <Select
            label="Key Type"
            options={[
              { value: "true", label: "Resident (discoverable) - Recommended" },
              { value: "false", label: "Non-resident (requires key file)" },
            ]}
            value={resident ? "true" : "false"}
            onChange={(e) => setResident(e.target.value === "true")}
            disabled={generating || generatedKey !== null}
          />

          {!generatedKey ? (
            <Button
              variant="primary"
              size="sm"
              onClick={handleGenerateKey}
              loading={generating}
              disabled={yubikeys.length === 0 || !yubikeys.some((yk) => yk.fido2_supported) || keyName.trim() === ""}
            >
              <KeyRound size={14} />
              Generate Key
            </Button>
          ) : (
            <div className="flex items-center gap-2 p-2 rounded-lg bg-green-50 dark:bg-green-900/10 border border-green-200 dark:border-green-800">
              <CheckCircle2 size={14} className="text-green-500 shrink-0" />
              <div className="min-w-0">
                <span className="text-sm font-medium text-green-700 dark:text-green-400">
                  Key generated: {generatedKey.name}
                </span>
                <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark truncate">
                  {generatedKey.path}
                </p>
              </div>
            </div>
          )}
        </div>

        {generating && (
          <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
            Touch your YubiKey when it blinks to confirm key generation...
          </p>
        )}
      </div>
    </div>
  );

  // ── Render: Deploy step ────────────────────────────────────────────

  const renderDeploy = () => (
    <div className="space-y-4">
      <div className="p-4 rounded-lg border border-border-light dark:border-border-dark space-y-3">
        <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
          Deploy Public Key to {node.name}
        </span>

        <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
          The generated public key will be added to the remote node's <code className="px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-800 text-xs">authorized_keys</code>.
          The existing password authentication will be used for this deployment.
        </p>

        {deploying && (
          <div className="flex items-center gap-2 p-3 rounded-lg bg-warm-50 dark:bg-warm-900/10 border border-warm-200 dark:border-warm-800">
            <Loader2 size={16} className="animate-spin text-warm-500" />
            <span className="text-sm text-warm-700 dark:text-warm-300">
              Deploying key to remote node...
            </span>
          </div>
        )}

        {deployed && (
          <div className="flex items-center gap-2 p-3 rounded-lg bg-green-50 dark:bg-green-900/10 border border-green-200 dark:border-green-800">
            <CheckCircle2 size={16} className="text-green-500" />
            <span className="text-sm text-green-700 dark:text-green-400">
              Public key deployed successfully
            </span>
          </div>
        )}

        {deployError && (
          <div className="space-y-2">
            <div className="flex items-center gap-2 p-3 rounded-lg bg-red-50 dark:bg-red-900/10 border border-red-200 dark:border-red-800">
              <XCircle size={16} className="text-red-500" />
              <span className="text-sm text-red-700 dark:text-red-400">{deployError}</span>
            </div>
            <Button variant="secondary" size="sm" onClick={handleDeploy}>
              Retry Deployment
            </Button>
          </div>
        )}
      </div>
    </div>
  );

  // ── Render: Finalize step ──────────────────────────────────────────

  const renderFinalize = () => (
    <div className="space-y-4">
      <div className="p-4 rounded-lg border border-border-light dark:border-border-dark space-y-3">
        <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
          Switch Authentication Method
        </span>
        <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark">
          Update {node.name}'s configuration to use YubiKey (FIDO2) authentication instead of password.
        </p>

        {!switched && (
          <label className="flex items-start gap-3 p-3 rounded-lg border border-border-light dark:border-border-dark cursor-pointer hover:bg-warm-50 dark:hover:bg-warm-900/10 transition-colors">
            <input
              type="checkbox"
              checked={hardenSshd}
              onChange={(e) => setHardenSshd(e.target.checked)}
              disabled={switching}
              className="mt-0.5 rounded border-gray-300 text-warm-500 focus:ring-warm-300/50"
            />
            <div>
              <span className="text-sm font-medium text-text-primary-light dark:text-text-primary-dark">
                Harden remote SSH (recommended)
              </span>
              <p className="text-xs text-text-secondary-light dark:text-text-secondary-dark mt-0.5">
                Disable password authentication on the remote node to require security key login.
                A backup of the current sshd_config will be created before any changes.
              </p>
            </div>
          </label>
        )}

        {!switched ? (
          <Button
            variant="primary"
            size="sm"
            onClick={handleSwitchAuth}
            loading={switching || hardening}
          >
            <ShieldCheck size={14} />
            {hardenSshd ? "Switch & Harden" : "Switch to YubiKey"}
          </Button>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center gap-2 p-3 rounded-lg bg-green-50 dark:bg-green-900/10 border border-green-200 dark:border-green-800">
              <CheckCircle2 size={16} className="text-green-500" />
              <span className="text-sm text-green-700 dark:text-green-400">
                Authentication switched to YubiKey
              </span>
            </div>
            {hardenResult && (
              <div className="p-3 rounded-lg bg-green-50 dark:bg-green-900/10 border border-green-200 dark:border-green-800 space-y-1">
                <div className="flex items-center gap-2">
                  <CheckCircle2 size={14} className="text-green-500" />
                  <span className="text-sm text-green-700 dark:text-green-400">SSH hardened</span>
                </div>
                {hardenResult.changes_made.map((change, i) => (
                  <p key={i} className="text-xs text-text-secondary-light dark:text-text-secondary-dark ml-6">
                    {change}
                  </p>
                ))}
                {hardenResult.warnings.length > 0 && hardenResult.warnings.map((w, i) => (
                  <div key={i} className="flex items-center gap-1 ml-6">
                    <AlertTriangle size={11} className="text-yellow-500" />
                    <span className="text-xs text-yellow-600 dark:text-yellow-400">{w}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );

  // ── Render: Main ───────────────────────────────────────────────────

  const renderStepContent = () => {
    switch (currentStep) {
      case "prerequisites":
        return renderPrerequisites();
      case "generate":
        return renderGenerate();
      case "deploy":
        return renderDeploy();
      case "finalize":
        return renderFinalize();
    }
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Upgrade to YubiKey Authentication"
      className="max-w-2xl"
    >
      {renderStepIndicator()}
      {renderStepContent()}

      {/* Navigation buttons */}
      <div className="flex items-center justify-between pt-4 mt-4 border-t border-border-light dark:border-border-dark">
        <Button
          variant="ghost"
          size="sm"
          onClick={goBack}
          disabled={stepIndex === 0}
        >
          Back
        </Button>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={onClose}>
            Cancel
          </Button>
          {currentStep === "finalize" && switched ? (
            <Button variant="primary" size="sm" onClick={onClose}>
              Done
            </Button>
          ) : (
            <Button
              variant="primary"
              size="sm"
              onClick={goNext}
              disabled={!canProceed()}
            >
              Continue
              <ChevronRight size={14} />
            </Button>
          )}
        </div>
      </div>
    </Dialog>
  );
}
