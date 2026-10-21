import { Banner } from "@/components/banner";
import { hostsSwitchEnvironment } from "@/lib/hosven";
import { useStore } from "@/store";

/**
 * More than one environment has entries on, after a hand edit or a file an
 * older version wrote. Each action keeps one environment and writes the file.
 */
export function EnvironmentConflictBanner({ ids }: { ids: string[] }) {
  const hosts = useStore((s) => s.hosts);
  const run = useStore((s) => s.run);
  const applying = hosts?.applying ?? false;
  const environments = ids.flatMap((id) => hosts?.profiles.filter((p) => p.id === id) ?? []);

  return (
    <Banner
      tone="warning"
      title={`${environments.length} môi trường đang cùng bật: ${environments.map((p) => p.name).join(", ")}`}
      actions={environments.map((p) => ({
        label: `Chỉ giữ ${p.name}`,
        onClick: () => void run(hostsSwitchEnvironment(p.id, "ask")),
        disabled: applying,
      }))}
    >
      Hostname trùng nhau sẽ trỏ theo dòng đứng trước trong /etc/hosts. Giữ một môi trường để tắt các môi trường còn
      lại.
    </Banner>
  );
}
