import { useAppArmor } from "../hooks/useAppArmor";
import { AppArmorDashboard } from "../components/apparmor/AppArmorDashboard";
export function AppArmorPage() {
  const appArmor = useAppArmor();

  return (
    <div>
      <div className="mb-6">
        <h1 className="text-xl font-semibold text-text-primary-light dark:text-text-primary-dark">
          AppArmor Manager
        </h1>
        <p className="text-sm text-text-secondary-light dark:text-text-secondary-dark mt-1">
          Monitor and manage AppArmor profiles and permissions
        </p>
      </div>
      <AppArmorDashboard {...appArmor} />
    </div>
  );
}
