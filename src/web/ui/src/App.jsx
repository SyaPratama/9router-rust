import { BrowserRouter, Navigate, Route, Routes, useParams } from 'react-router-dom';
import AuthGate from './AuthGate.jsx';
import { RuntimeI18nProvider } from './upstream/i18n/RuntimeI18nProvider';
import { ThemeProvider } from './upstream/shared/components/ThemeProvider';
import DashboardLayout from './upstream/shared/components/layouts/DashboardLayout';
import BasicChatPage from './upstream/app/(dashboard)/dashboard/basic-chat/BasicChatPageClient';
import CLIToolsPage from './upstream/app/(dashboard)/dashboard/cli-tools/CLIToolsPageClient';
import ToolDetailPage from './upstream/app/(dashboard)/dashboard/cli-tools/[toolId]/ToolDetailClient';
import ConsoleLogPage from './upstream/app/(dashboard)/dashboard/console-log/ConsoleLogClient';
import CombosPage from './upstream/app/(dashboard)/dashboard/combos/page';
import ComboDetailPage from './upstream/app/(dashboard)/dashboard/media-providers/combo/[id]/page';
import EndpointPage from './upstream/app/(dashboard)/dashboard/endpoint/EndpointPageClient';
import MediaProvidersPage from './upstream/app/(dashboard)/dashboard/media-providers/[kind]/page';
import MediaProviderDetailPage from './upstream/app/(dashboard)/dashboard/media-providers/[kind]/[id]/page';
import MediaProvidersWebPage from './upstream/app/(dashboard)/dashboard/media-providers/web/page';
import MitmPage from './upstream/app/(dashboard)/dashboard/mitm/MitmPageClient';
import ProfilePage from './upstream/app/(dashboard)/dashboard/profile/page';
import ProvidersPage from './upstream/app/(dashboard)/dashboard/providers/page';
import NewProviderPage from './upstream/app/(dashboard)/dashboard/providers/new/page';
import ProviderDetailPage from './upstream/app/(dashboard)/dashboard/providers/[id]/page';
import ProxyPoolsPage from './upstream/app/(dashboard)/dashboard/proxy-pools/page';
import PxpipePage from './upstream/app/(dashboard)/dashboard/pxpipe/PxpipeClient';
import QuotaPage from './upstream/app/(dashboard)/dashboard/quota/page';
import SkillsPage from './upstream/app/(dashboard)/dashboard/skills/page';
import TokenSaverPage from './upstream/app/(dashboard)/dashboard/token-saver/TokenSaverClient';
import TranslatorPage from './upstream/app/(dashboard)/dashboard/translator/page';
import UsagePage from './upstream/app/(dashboard)/dashboard/usage/page';

function Endpoint() {
  return <EndpointPage machineId="" />;
}

function CLITools() {
  return <CLIToolsPage machineId="" />;
}

function ToolDetail() {
  const { toolId } = useParams();
  return <ToolDetailPage toolId={toolId} machineId="" />;
}

function DashboardRoutes() {
  return (
    <DashboardLayout>
      <Routes>
        <Route path="/dashboard" element={<Endpoint />} />
        <Route path="/dashboard/endpoint/*" element={<Endpoint />} />
        <Route path="/dashboard/providers" element={<ProvidersPage />} />
        <Route path="/dashboard/providers/new" element={<NewProviderPage />} />
        <Route path="/dashboard/providers/:id" element={<ProviderDetailPage />} />
        <Route path="/dashboard/combos" element={<CombosPage />} />
        <Route path="/dashboard/usage/*" element={<UsagePage />} />
        <Route path="/dashboard/quota" element={<QuotaPage />} />
        <Route path="/dashboard/token-saver" element={<TokenSaverPage />} />
        <Route path="/dashboard/cli-tools" element={<CLITools />} />
        <Route path="/dashboard/cli-tools/:toolId" element={<ToolDetail />} />
        <Route path="/dashboard/console-log" element={<ConsoleLogPage />} />
        <Route path="/dashboard/translator" element={<TranslatorPage />} />
        <Route path="/dashboard/skills" element={<SkillsPage />} />
        <Route path="/dashboard/proxy-pools" element={<ProxyPoolsPage />} />
        <Route path="/dashboard/media-providers/web" element={<MediaProvidersWebPage />} />
        <Route path="/dashboard/media-providers/combo/:id" element={<ComboDetailPage />} />
        <Route path="/dashboard/media-providers/:kind/:id" element={<MediaProviderDetailPage />} />
        <Route path="/dashboard/media-providers/:kind" element={<MediaProvidersPage />} />
        <Route path="/dashboard/mitm" element={<MitmPage />} />
        <Route path="/dashboard/profile" element={<ProfilePage />} />
        <Route path="/dashboard/basic-chat" element={<BasicChatPage />} />
        <Route path="/dashboard/pxpipe" element={<PxpipePage />} />
        <Route path="*" element={<Navigate to="/dashboard" replace />} />
      </Routes>
    </DashboardLayout>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <ThemeProvider>
        <RuntimeI18nProvider>
          <AuthGate>
            <DashboardRoutes />
          </AuthGate>
        </RuntimeI18nProvider>
      </ThemeProvider>
    </BrowserRouter>
  );
}
