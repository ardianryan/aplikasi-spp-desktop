import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./App.css";

interface UserSession {
  token: string;
  nama: string;
  role: string;
}

// Interface Definitions
interface StudentInfo {
  id: string;
  nis: string | null;
  nisn: string | null;
  nama: string;
  kelas: string | null;
}

interface BillSummary {
  total_tagihan: number;
  total_dibayar: number;
  total_keringanan: number;
  total_tunggakan: number;
}

interface PaymentAssignment {
  id: string;
  payment_type_name: string;
  payment_type_type: string;
  month: number | null;
  month_name: string | null;
  amount: number;
  paid_amount: number;
  relief_amount: number;
  remaining_amount: number;
  status: string;
  academic_year_id?: string;
  academic_year_name?: string;
}

interface PaymentHistory {
  id: string;
  transaction_code: string;
  total_amount: number;
  payment_method_name: string;
  notes: string | null;
  created_at: string;
}

interface StudentDetails {
  student: StudentInfo;
  summary: BillSummary;
  assignments: PaymentAssignment[];
  payments: PaymentHistory[];
  savings_balance: number;
  unpaid_years: string[];
  active_year_id: string | null;
}

interface PaymentCartItem {
  assignmentId: string;
  name: string;
  remaining: number;
  payAmount: number;
}

// ==========================================
// 🎨 Modern SVG Icon Components (No Emoji)
// ==========================================
const ActivationIcon = () => (
  <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ color: "var(--primary)" }}>
    <path d="M18 11.034V20a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2h1.034" />
    <path d="M14 2h8v8" />
    <path d="M22 2l-10 10" />
    <path d="M9 13.034V20a2 2 0 0 0 2 2h3a2 2 0 0 0 2-2v-5" />
  </svg>
);

const KeyIcon = () => (
  <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ color: "var(--primary)" }}>
    <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
  </svg>
);

const LogoIcon = () => (
  <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ color: "var(--primary)" }}>
    <rect x="4" y="2" width="16" height="20" rx="2" ry="2" />
    <line x1="9" y1="22" x2="9" y2="16" />
    <line x1="15" y1="22" x2="15" y2="16" />
    <line x1="9" y1="16" x2="15" y2="16" />
    <path d="M9 6h6M9 10h6" />
  </svg>
);

const MonitorIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ verticalAlign: "middle" }}>
    <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
    <line x1="8" y1="21" x2="16" y2="21" />
    <line x1="12" y1="17" x2="12" y2="21" />
  </svg>
);

const SavingsIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ verticalAlign: "middle" }}>
    <line x1="12" y1="1" x2="12" y2="23" />
    <path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
  </svg>
);

const SyncIcon = ({ className = "", size = 18 }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={{ verticalAlign: "middle" }}>
    <path d="M21.5 2v6h-6" />
    <path d="M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67" />
  </svg>
);

const UserIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ verticalAlign: "middle", marginRight: "6px" }}>
    <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
    <circle cx="12" cy="7" r="4" />
  </svg>
);

const SearchIcon = ({ size = 16, className = "" }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className={className}>
    <circle cx="11" cy="11" r="8" />
    <line x1="21" y1="21" x2="16.65" y2="16.65" />
  </svg>
);

const StudentIcon = ({ size = 28 }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ color: "var(--primary)" }}>
    <path d="M22 10v6M2 10l10-5 10 5-10 5z" />
    <path d="M6 12v5c0 2 2 3 6 3s6-1 6-3v-5" />
  </svg>
);

const CloseIcon = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ verticalAlign: "middle" }}>
    <line x1="18" y1="6" x2="6" y2="18" />
    <line x1="6" y1="6" x2="18" y2="18" />
  </svg>
);

const PlugIcon = ({ size = 18 }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M18 10h-1.5c-.8 0-1.5-.7-1.5-1.5V7c0-.8-.7-1.5-1.5-1.5H10c-.8 0-1.5.7-1.5 1.5v1.5C8.5 9.3 7.8 10 7 10H6c-1.1 0-2 .9-2 2v2c0 1.1.9 2 2 2h1c.8 0 1.5.7 1.5 1.5V19c0 .8.7 1.5 1.5 1.5h3.5c.8 0 1.5-.7 1.5-1.5v-1.5c0-.8.7-1.5 1.5-1.5H18c1.1 0 2-.9 2-2v-2c0-1.1-.9-2-2-2z" />
  </svg>
);

const DepositIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ marginRight: "8px" }}>
    <line x1="12" y1="5" x2="12" y2="19" />
    <polyline points="19 12 12 19 5 12" />
  </svg>
);

const WithdrawIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" style={{ marginRight: "8px" }}>
    <line x1="12" y1="19" x2="12" y2="5" />
    <polyline points="5 12 12 5 19 12" />
  </svg>
);

const GlobeIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ marginRight: "8px", verticalAlign: "middle" }}>
    <circle cx="12" cy="12" r="10" />
    <line x1="2" y1="12" x2="22" y2="12" />
    <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
  </svg>
);

const ClockIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ marginRight: "8px", verticalAlign: "middle" }}>
    <circle cx="12" cy="12" r="10" />
    <polyline points="12 6 12 12 16 14" />
  </svg>
);

const InfoIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ marginRight: "6px", verticalAlign: "middle" }}>
    <circle cx="12" cy="12" r="10" />
    <line x1="12" y1="16" x2="12" y2="12" />
    <line x1="12" y1="8" x2="12.01" y2="8" />
  </svg>
);

const PrintIcon = ({ size = 16, className = "" }) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    style={{ verticalAlign: "middle" }}
  >
    <polyline points="6 9 6 2 18 2 18 9" />
    <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
    <rect x="6" y="14" width="12" height="8" />
  </svg>
);

const HistoryIcon = () => (
  <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ marginRight: "8px", verticalAlign: "middle" }}>
    <circle cx="12" cy="12" r="10" />
    <polyline points="12 6 12 12 16 14" />
  </svg>
);

const ReportsIcon = () => (
  <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ marginRight: "8px", verticalAlign: "middle" }}>
    <line x1="18" y1="20" x2="18" y2="10" />
    <line x1="12" y1="20" x2="12" y2="4" />
    <line x1="6" y1="20" x2="6" y2="14" />
  </svg>
);

function App() {
  // Navigation State
  const [activeTab, setActiveTab] = useState<"pos" | "savings" | "journals" | "sync" | "history" | "reports">("pos");
  const [isOnline, setIsOnline] = useState(navigator.onLine);

  // Onboarding Wizard State
  const [onboardingStep, setOnboardingStep] = useState<1 | 2 | 3>(1);
  const [isFreshInstall, setIsFreshInstall] = useState<boolean | null>(null); // null = loading
  const [onboardingConnTested, setOnboardingConnTested] = useState(false);
  const [onboardingConnMsg, setOnboardingConnMsg] = useState("");
  const [isTestingOnboardConn, setIsTestingOnboardConn] = useState(false);

  // Global Config
  const [academicYearId, setAcademicYearId] = useState<string | null>(null);
  const [academicYearName, setAcademicYearName] = useState<string | null>(null);
  const [loketId, setLoketId] = useState<string>("LOK-01");
  const [savingsEnabled, setSavingsEnabled] = useState<boolean>(false);
  const [expandedYears, setExpandedYears] = useState<{ [key: string]: boolean }>({});
  const [schoolSettings, setSchoolSettings] = useState<{ [key: string]: string }>({});
  const [printReceiptData, setPrintReceiptData] = useState<any | null>(null);
  const [showReceiptModal, setShowReceiptModal] = useState(false);

  // Derived: custom terminology from settings (fallback to default Indonesian)
  const terms = {
    pembayaran:  schoolSettings.term_pembayaran  || "Pembayaran",
    tagihan:     schoolSettings.term_tagihan     || "Tagihan",
    spp:         schoolSettings.term_spp         || "SPP",
    siswa:       schoolSettings.term_siswa       || "Siswa",
    sumbangan:   schoolSettings.term_sumbangan   || "Sumbangan",
    iuran:       schoolSettings.term_iuran       || "Iuran",
    kwitansi:    schoolSettings.term_kwitansi    || "Kwitansi",
    tunggakan:   schoolSettings.term_tunggakan   || "Tunggakan",
  };

  // Derived: nominal visibility settings
  const showNominal = {
    receipt:      schoolSettings.show_nominal_receipt      !== "0",
    receiptBebas: schoolSettings.show_nominal_receipt_bebas !== "0",
    portal:       schoolSettings.show_nominal_portal       !== "0",
  };

  // Search & Student Selection State
  const [searchQuery, setSearchQuery] = useState("");
  const [studentStatus, setStudentStatus] = useState<"active" | "inactive">("active");
  const [searchResults, setSearchResults] = useState<StudentInfo[]>([]);
  const [selectedStudent, setSelectedStudent] = useState<StudentInfo | null>(null);
  const [studentDetails, setStudentDetails] = useState<StudentDetails | null>(null);
  const [loadingDetails, setLoadingDetails] = useState(false);

  // Global History & Reports state
  const [historySearchQuery, setHistorySearchQuery] = useState("");
  const [historyPage, setHistoryPage] = useState(1);
  const [historyList, setHistoryList] = useState<any[]>([]);
  const [reportsData, setReportsData] = useState<any | null>(null);

  // POS / Cashier Cart State
  const [cart, setCart] = useState<PaymentCartItem[]>([]);
  const [paymentMethodName, setPaymentMethodName] = useState("Tunai");
  const [paymentNote, setPaymentNote] = useState("");
  const [isProcessingPayment, setIsProcessingPayment] = useState(false);
  const [paymentSuccessMsg, setPaymentSuccessMsg] = useState<string | null>(null);

  // Savings (Tabungan) State
  const [savingsAmount, setSavingsAmount] = useState<number>(0);
  const [savingsType, setSavingsType] = useState<"deposit" | "withdraw">("deposit");
  const [savingsNote, setSavingsNote] = useState("");
  const [isProcessingSavings, setIsProcessingSavings] = useState(false);

  // Sync Settings & Logs State
  const [apiUrl, setApiUrl] = useState("http://localhost:8000");
  const [apiKey, setApiKey] = useState("");
  const [syncStatusMsg, setSyncStatusMsg] = useState<string | null>(null);
  const [isSyncing, setIsSyncing] = useState(false);
  const [lastSyncTime, setLastSyncTime] = useState<string>("Belum pernah");

  // Session State
  const [session, setSession] = useState<UserSession | null>(null);
  const [manualToken, setManualToken] = useState("");
  const [isEditingActivation, setIsEditingActivation] = useState(false);
  const [isActivated, setIsActivated] = useState(false);
  const [isTestingConnection, setIsTestingConnection] = useState(false);

  // Today Cashier stats and history
  const [todayStats, setTodayStats] = useState({ today_total: 0, monthly_total: 0, today_count: 0 });
  const [recentTransactions, setRecentTransactions] = useState<any[]>([]);

  async function loadTodayStatsAndHistory() {
    try {
      const stats = await invoke<any>("get_today_cashier_stats");
      setTodayStats(stats);
      const list = await invoke<any[]>("get_today_payments");
      setRecentTransactions(list);
    } catch (err) {
      console.error("Gagal memuat statistik kasir hari ini:", err);
    }
  }

  // Load dynamic online/offline state
  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);
    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);
    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, []);

  // Silent sync helper
  async function runSilentSync() {
    if (!navigator.onLine) return;
    try {
      const msg = await invoke<string>("trigger_sync");
      console.log("Auto-Sync Success:", msg);
      setLastSyncTime(new Date().toLocaleString("id-ID"));
      await loadTodayStatsAndHistory();
      await loadHistory(historyPage, historySearchQuery, false);
      if (selectedStudent) {
        await reloadCurrentStudent();
      }
    } catch (err) {
      console.warn("Auto-Sync failed:", err);
    }
  }

  // Auto-sync interval (every 45 seconds when online)
  useEffect(() => {
    const interval = setInterval(() => {
      if (isOnline) {
        runSilentSync();
      }
    }, 45000);
    return () => clearInterval(interval);
  }, [isOnline, historyPage, historySearchQuery, selectedStudent]);

  // Sync immediately when back online
  useEffect(() => {
    if (isOnline) {
      runSilentSync();
    }
  }, [isOnline]);

  // Redirect kasir if they try to access savings tab, or if savings is disabled
  useEffect(() => {
    if (session) {
      if ((session.role === "kasir" || !savingsEnabled) && activeTab === "savings") {
        setActiveTab("pos");
      }
    }
  }, [session, activeTab, savingsEnabled]);

  // Auto-expand active academic year group on student change
  useEffect(() => {
    if (studentDetails) {
      const activeId = studentDetails.active_year_id || "none";
      setExpandedYears({ [activeId]: true });
    } else {
      setExpandedYears({});
    }
  }, [studentDetails]);

  // Load Active Academic Year & Sync Config & Session on mount
  useEffect(() => {
    loadInitialConfig();

    // Listen to Deep Link authentication redirect from browser
    const unlisten = listen<string>("desktop-login-success", async (event) => {
      console.log("Deep link payload received:", event.payload);
      try {
        const url = new URL(event.payload);
        const token = url.searchParams.get("token");
        const nama = url.searchParams.get("nama");
        const role = url.searchParams.get("role");
        if (token && nama && role) {
          const sess = await invoke<UserSession>("save_session", { token, nama, role });
          setSession(sess);
        }
      } catch (err) {
        console.error("Gagal memproses SSO Deep Link:", err);
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  // Load History
  async function loadHistory(page: number, search: string, append = false) {
    try {
      const data = await invoke<any[]>("get_all_payment_history", { page, search });
      if (append) {
        setHistoryList((prev) => [...prev, ...data]);
      } else {
        setHistoryList(data);
      }
      setHistoryPage(page);
    } catch (err) {
      console.error("Gagal mengambil riwayat pembayaran:", err);
    }
  }

  // Load Reports
  async function loadReports() {
    try {
      const data = await invoke<any>("get_local_reports");
      setReportsData(data);
    } catch (err) {
      console.error("Gagal mengambil laporan kas lokal:", err);
    }
  }

  // Load global history or reports data when activeTab changes
  useEffect(() => {
    if (activeTab === "history") {
      loadHistory(1, historySearchQuery, false);
    } else if (activeTab === "reports") {
      loadReports();
    }
  }, [activeTab]);

  // Handle history search debouncing
  useEffect(() => {
    if (activeTab === "history") {
      const delay = setTimeout(() => {
        loadHistory(1, historySearchQuery, false);
      }, 300);
      return () => clearTimeout(delay);
    }
  }, [historySearchQuery]);

  async function loadInitialConfig() {
    try {
      const yearData = await invoke<[string, string] | null>("get_active_academic_year");
      if (yearData) {
        setAcademicYearId(yearData[0]);
        setAcademicYearName(yearData[1]);
      }

      const freshInstall = await invoke<boolean>("is_fresh_install").catch(() => false);
      setIsFreshInstall(freshInstall);
      if (freshInstall) return; // skip rest until onboarding done

      const enabled = await invoke<boolean>("is_savings_enabled").catch(() => false);
      setSavingsEnabled(enabled);

      const schoolSet = await invoke<{ [key: string]: string }>("get_school_settings");
      setSchoolSettings(schoolSet);

      const [url, key] = await invoke<[string, string]>("get_sync_config");
      if (url && url.trim().length > 0 && key && key.trim().length > 0) {
        setApiUrl(url);
        setApiKey(key);
        setIsActivated(true);
      } else {
        setApiUrl(url || "http://localhost:8000");
        setApiKey(key || "");
        setIsActivated(false);
      }

      const activeSess = await invoke<UserSession | null>("get_active_session");
      if (activeSess) {
        setSession(activeSess);
      }

      await loadTodayStatsAndHistory();
    } catch (err) {
      console.error("Gagal memuat konfigurasi awal:", err);
    }
  }

  async function openLogin() {
    try {
      const cleanUrl = apiUrl.replace(/\/+$/, "");
      await openUrl(`${cleanUrl}/auth/desktop-login`);
    } catch (err) {
      alert("Silakan buka browser Anda ke URL ini untuk login:\n" + apiUrl + "/auth/desktop-login");
    }
  }

  async function handleLogout() {
    try {
      await invoke("logout");
      setSession(null);
      setSelectedStudent(null);
      setStudentDetails(null);
    } catch (err) {
      alert("Gagal melakukan logout: " + err);
    }
  }

  async function handleManualLogin(e: React.FormEvent) {
    e.preventDefault();
    if (!manualToken.trim()) return;
    try {
      let token = manualToken.trim();
      let nama = "Admin";
      let role = "admin";

      if (token.startsWith("partisipasi-sekolah://")) {
        const urlStr = token.replace("partisipasi-sekolah://", "http://");
        const url = new URL(urlStr);
        token = url.searchParams.get("token") || token;
        nama = decodeURIComponent(url.searchParams.get("nama") || nama);
        role = url.searchParams.get("role") || role;
      }

      const sess = await invoke<UserSession>("save_session", { token, nama, role });
      setSession(sess);
      setManualToken("");
      alert("Login berhasil!");
    } catch (err) {
      alert("Gagal melakukan login manual: " + err);
    }
  }

  async function handleActivate(e: React.FormEvent) {
    e.preventDefault();
    if (!apiUrl.trim() || !apiKey.trim()) {
      alert("Mohon isi URL Endpoint dan API Key!");
      return;
    }

    setIsTestingConnection(true);

    try {
      // 1. Test connection to server with provided API URL and API Key
      await invoke("test_sync_connection", { apiUrl: apiUrl.trim(), apiKey: apiKey.trim() });
      
      // 2. If test successful, save configurations
      await invoke("set_sync_config", { apiUrl: apiUrl.trim(), apiKey: apiKey.trim() });
      setApiKey(apiKey.trim());
      setIsActivated(true);
      setIsEditingActivation(false);
      alert("Koneksi berhasil terhubung! Silakan masuk via SSO.");
    } catch (err) {
      alert("Gagal terhubung ke server:\n" + err);
    } finally {
      setIsTestingConnection(false);
    }
  }

  // Trigger search on query change
  useEffect(() => {
    if (searchQuery.trim().length >= 2) {
      const delayDebounce = setTimeout(() => {
        performSearch();
      }, 300);
      return () => clearTimeout(delayDebounce);
    } else {
      setSearchResults([]);
    }
  }, [searchQuery, studentStatus]);

  async function performSearch() {
    try {
      const list = await invoke<StudentInfo[]>("search_students", { 
        query: searchQuery, 
        status: studentStatus 
      });
      setSearchResults(list);
    } catch (err) {
      console.error(err);
    }
  }

  // Load selected student details
  async function selectStudent(student: StudentInfo) {
    setSelectedStudent(student);
    setSearchQuery("");
    setSearchResults([]);
    setCart([]);
    setPaymentSuccessMsg(null);
    setLoadingDetails(true);
    try {
      const details = await invoke<StudentDetails>("get_student_details", { studentId: student.id });
      setStudentDetails(details);
    } catch (err) {
      alert("Gagal memuat detail siswa: " + err);
    } finally {
      setLoadingDetails(false);
    }
  }

  // Reload details for current student
  async function reloadCurrentStudent() {
    if (!selectedStudent) return;
    try {
      const details = await invoke<StudentDetails>("get_student_details", { studentId: selectedStudent.id });
      setStudentDetails(details);
    } catch (err) {
      console.error("Reload gagal:", err);
    }
  }

  // Add / edit cart items
  const handleCartChange = (assignment: PaymentAssignment, payAmount: number) => {
    if (payAmount <= 0) {
      setCart(cart.filter(item => item.assignmentId !== assignment.id));
    } else {
      const existing = cart.find(item => item.assignmentId === assignment.id);
      if (existing) {
        setCart(cart.map(item => item.assignmentId === assignment.id ? { ...item, payAmount } : item));
      } else {
        setCart([...cart, {
          assignmentId: assignment.id,
          name: assignment.payment_type_name + (assignment.month_name ? ` (${assignment.month_name})` : ""),
          remaining: assignment.remaining_amount,
          payAmount
        }]);
      }
    }
  };

  // Submit payment
  async function processPayment(e: React.FormEvent) {
    e.preventDefault();
    if (!selectedStudent || cart.length === 0 || !academicYearId) return;

    setIsProcessingPayment(true);
    setPaymentSuccessMsg(null);

    const items = cart.map(item => ({
      payment_assignment_id: item.assignmentId,
      amount: item.payAmount,
      payment_type_name: item.name
    }));

    const totalAmount = cart.reduce((acc, item) => acc + item.payAmount, 0);

    try {
      const trxCode = await invoke<string>("save_payment", {
        studentId: selectedStudent.id,
        academicYearId,
        totalAmount,
        paymentMethodId: null,
        paymentMethodName,
        notes: paymentNote || null,
        items,
        loketId
      });
      setPaymentSuccessMsg(`Transaksi Berhasil! Kode Nota: ${trxCode}`);
      setCart([]);
      setPaymentNote("");
      await reloadCurrentStudent();
      await loadTodayStatsAndHistory();
      
      // Auto-trigger printing receipt
      try {
        const receipt = await invoke<any>("get_payment_receipt", { transactionCode: trxCode });
        setPrintReceiptData(receipt);
        setShowReceiptModal(true);
      } catch (err) {
        console.error("Gagal memuat kwitansi otomatis:", err);
      }
    } catch (err) {
      alert("Gagal memproses pembayaran: " + err);
    } finally {
      setIsProcessingPayment(false);
    }
  }

  // Trigger receipt printing manual
  async function triggerReceiptPrint(trxCode: string) {
    try {
      const receipt = await invoke<any>("get_payment_receipt", { transactionCode: trxCode });
      setPrintReceiptData(receipt);
      setShowReceiptModal(true);
    } catch (err) {
      alert("Gagal memuat data kwitansi: " + err);
    }
  }

  // Process savings transaction
  async function handleSavingsTransaction(e: React.FormEvent) {
    e.preventDefault();
    if (!selectedStudent || savingsAmount <= 0 || !academicYearId) return;

    setIsProcessingSavings(true);
    try {
      const newBalance = await invoke<number>("save_savings_transaction", {
        studentId: selectedStudent.id,
        academicYearId,
        transactionType: savingsType,
        amount: savingsAmount,
        notes: savingsNote || null
      });
      alert(`Transaksi Tabungan Berhasil! Saldo Baru: Rp ${newBalance.toLocaleString("id-ID")}`);
      setSavingsAmount(0);
      setSavingsNote("");
      await reloadCurrentStudent();
    } catch (err) {
      alert("Gagal memproses transaksi tabungan: " + err);
    } finally {
      setIsProcessingSavings(false);
    }
  }

  // Save Settings
  async function saveConfig(e: React.FormEvent) {
    e.preventDefault();
    try {
      await invoke("set_sync_config", { apiUrl, apiKey });
      alert("Pengaturan sinkronisasi berhasil disimpan.");
    } catch (err) {
      alert("Gagal menyimpan pengaturan: " + err);
    }
  }

  // Manual trigger sync
  async function runSync() {
    setIsSyncing(true);
    setSyncStatusMsg("Sedang melakukan sinkronisasi dengan server...");
    try {
      const msg = await invoke<string>("trigger_sync");
      setSyncStatusMsg(msg);
      // Ambil waktu sekarang
      setLastSyncTime(new Date().toLocaleString("id-ID"));
      
      // Ambil ulang tahun akademik aktif setelah sinkronisasi berhasil
      const yearData = await invoke<[string, string] | null>("get_active_academic_year");
      if (yearData) {
        setAcademicYearId(yearData[0]);
        setAcademicYearName(yearData[1]);
      }

      const enabled = await invoke<boolean>("is_savings_enabled");
      setSavingsEnabled(enabled);

      await loadTodayStatsAndHistory();
      await loadHistory(historyPage, historySearchQuery, false);
      if (selectedStudent) {
        await reloadCurrentStudent();
      }
      // Re-download logo kop kwitansi setelah sync (mungkin ada update dari admin)
      invoke<string>("download_school_assets").then(async () => {
        const schoolSet = await invoke<{ [key: string]: string }>("get_school_settings");
        setSchoolSettings(schoolSet);
      }).catch(() => {});
      alert("Sinkronisasi berhasil!\n" + msg);
    } catch (err) {
      setSyncStatusMsg("Gagal melakukan sinkronisasi: " + err);
      alert("Gagal melakukan sinkronisasi:\n" + err);
    } finally {
      setIsSyncing(false);
    }
  }

  // ── Onboarding Wizard handler ──────────────────────────────────────────────
  async function handleOnboardingTestConn() {
    setIsTestingOnboardConn(true);
    setOnboardingConnMsg("");
    try {
      const result = await invoke<string>("test_sync_connection", { apiUrl, apiKey });
      setOnboardingConnMsg("✅ " + result);
      setOnboardingConnTested(true);
    } catch (err) {
      setOnboardingConnMsg("❌ Koneksi gagal: " + err);
      setOnboardingConnTested(false);
    } finally {
      setIsTestingOnboardConn(false);
    }
  }

  async function handleOnboardingFinish() {
    try {
      await invoke("set_sync_config", { apiUrl, apiKey });
      setIsActivated(true);
      setIsFreshInstall(false);
      // Langsung sync dari server untuk mendapatkan data & aset sekolah
      try {
        await invoke("trigger_sync");
        // Download logo kop kwitansi dan simpan sebagai base64 lokal
        await invoke<string>("download_school_assets").catch(() => {});
      } catch (_) {
        // Sync gagal saat onboarding tidak fatal, bisa sync manual nanti
      }
      await loadInitialConfig();
    } catch (err) {
      alert("Gagal menyimpan konfigurasi: " + err);
    }
  }

  // ── Loading screen (saat status fresh install sedang dicek) ─────────────────
  if (isFreshInstall === null) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100vh", background: "var(--bg-primary)" }}>
        <div style={{ textAlign: "center", color: "var(--text-muted)" }}>
          <div style={{ width: "40px", height: "40px", border: "3px solid var(--border-color)", borderTopColor: "var(--primary)", borderRadius: "50%", animation: "spin 0.8s linear infinite", margin: "0 auto 16px" }} />
          <p style={{ fontSize: "14px" }}>Memuat Partisipasi Sekolah…</p>
        </div>
      </div>
    );
  }

  // ── Onboarding Wizard (hanya muncul saat instalasi pertama) ─────────────────
  if (isFreshInstall) {
    const steps = [
      { num: 1, title: "Selamat Datang", desc: "Pengenalan Sistem" },
      { num: 2, title: "Koneksi Server", desc: "API Endpoint & Key" },
      { num: 3, title: "Aktivasi & Enkripsi", desc: "Setup Selesai" }
    ];

    const progressWidth = onboardingStep === 1 ? "33%" : onboardingStep === 2 ? "66%" : "100%";

    return (
      <div className="onboarding-screen-container">
        <div className="onboarding-card-wrapper">
          
          {/* App Brand Header */}
          <div className="onboarding-brand-header">
            <div className="app-logo-badge">
              <span className="logo-icon-box">
                <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M22 10v6M2 10l10-5 10 5-10 5z"/><path d="M6 12v5c3 3 9 3 12 0v-5"/>
                </svg>
              </span>
              <span className="logo-title">Partisipasi Sekolah</span>
              <span className="logo-subtitle">Offline Desktop POS</span>
            </div>
          </div>

          {/* Stepper Progress Indicator */}
          <div className="onboarding-stepper">
            <div className="stepper-track-bg">
              <div className="stepper-track-fill" style={{ width: progressWidth }} />
            </div>
            <div className="stepper-nodes">
              {steps.map((step) => {
                const isActive = onboardingStep === step.num;
                const isDone = onboardingStep > step.num;
                return (
                  <div key={step.num} className={`stepper-node ${isActive ? "active" : isDone ? "done" : ""}`}>
                    <div className="stepper-circle">
                      {isDone ? (
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                          <polyline points="20 6 9 17 4 12" />
                        </svg>
                      ) : (
                        <span>{step.num}</span>
                      )}
                    </div>
                    <div className="stepper-text">
                      <span className="stepper-title">{step.title}</span>
                      <span className="stepper-sub">{step.desc}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Main Content Card */}
          <div className="onboarding-main-card">
            
            {/* ── Step 1: Welcome Screen ── */}
            {onboardingStep === 1 && (
              <div className="onboarding-step-pane">
                <div className="pane-hero">
                  <div className="hero-icon-avatar">
                    <svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M22 10v6M2 10l10-5 10 5-10 5z"/>
                      <path d="M6 12v5c3 3 9 3 12 0v-5"/>
                    </svg>
                  </div>
                  <h2>Selamat Datang di Partisipasi Sekolah</h2>
                  <p>Aplikasi kasir POS & tabungan sekolah berbasis desktop. Bekerja offline dengan keamanan tingkat tinggi dan sinkronisasi otomatis ke server online.</p>
                </div>

                <div className="app-feature-grid">
                  <div className="feature-item">
                    <div className="feature-icon-box navy">
                      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
                        <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                      </svg>
                    </div>
                    <div className="feature-info">
                      <h4>Enkripsi SQLCipher AES-256</h4>
                      <p>Seluruh data lokal aman dari akses tanpa izin.</p>
                    </div>
                  </div>

                  <div className="feature-item">
                    <div className="feature-icon-box emerald">
                      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M5 12.55a11 11 0 0 1 14.08 0"/>
                        <path d="M1.42 9a16 16 0 0 1 21.16 0"/>
                        <path d="M8.53 16.11a6 6 0 0 1 6.95 0"/>
                        <line x1="12" y1="20" x2="12.01" y2="20"/>
                      </svg>
                    </div>
                    <div className="feature-info">
                      <h4>Sinkronisasi Otomatis</h4>
                      <p>Otomatis mengunggah transaksi saat online.</p>
                    </div>
                  </div>

                  <div className="feature-item">
                    <div className="feature-icon-box blue">
                      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="6 9 6 2 18 2 18 9"/>
                        <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/>
                        <rect x="6" y="14" width="12" height="8"/>
                      </svg>
                    </div>
                    <div className="feature-info">
                      <h4>Cetak Kwitansi Offline</h4>
                      <p>Kop sekolah & stempel tersimpan secara lokal.</p>
                    </div>
                  </div>

                  <div className="feature-item">
                    <div className="feature-icon-box amber">
                      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
                      </svg>
                    </div>
                    <div className="feature-info">
                      <h4>POS Kasir Cepat</h4>
                      <p>Proses transaksi bayar siswa tanpa jeda lag.</p>
                    </div>
                  </div>
                </div>

                <div className="pane-actions">
                  <button className="btn-primary" style={{ width: "100%", padding: "12px", fontSize: "14px", display: "inline-flex", alignItems: "center", justifyContent: "center", gap: "8px" }} onClick={() => setOnboardingStep(2)}>
                    <span>Mulai Setup Konfigurasi</span>
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/>
                    </svg>
                  </button>
                </div>
              </div>
            )}

            {/* ── Step 2: Server Config ── */}
            {onboardingStep === 2 && (
              <div className="onboarding-step-pane">
                <div className="pane-hero compact">
                  <h2>Konfigurasi Server API</h2>
                  <p>Masukkan API Server URL dan API Key terminal kasir yang terdaftar di panel admin online.</p>
                </div>

                <div className="form-group mb-3">
                  <label className="form-label font-bold">API Server Online URL (Endpoint)</label>
                  <input
                    type="text"
                    className="form-control"
                    value={apiUrl}
                    onChange={(e) => { setApiUrl(e.target.value); setOnboardingConnTested(false); setOnboardingConnMsg(""); }}
                    placeholder="https://partisipasi.sch.id"
                  />
                </div>

                <div className="form-group mb-3">
                  <label className="form-label font-bold">API Key Terminal</label>
                  <input
                    type="password"
                    className="form-control"
                    value={apiKey}
                    onChange={(e) => { setApiKey(e.target.value); setOnboardingConnTested(false); setOnboardingConnMsg(""); }}
                    placeholder="psk_xxxxxxxxxxxxxxxxxxxxxxxx"
                  />
                </div>

                {/* Status Box */}
                {onboardingConnMsg && (
                  <div className={`status-feedback-box ${onboardingConnTested ? "success" : "error"}`}>
                    <span className="feedback-icon">
                      {onboardingConnTested ? "✓" : "⚠"}
                    </span>
                    <span>{onboardingConnMsg}</span>
                  </div>
                )}

                <div className="pane-actions-row mt-4">
                  <button className="btn-secondary" onClick={() => setOnboardingStep(1)}>
                    ← Kembali
                  </button>

                  <button className="btn-secondary" onClick={handleOnboardingTestConn} disabled={isTestingOnboardConn || !apiUrl || !apiKey}>
                    {isTestingOnboardConn ? "Menguji..." : "🔗 Uji Koneksi"}
                  </button>

                  <button className="btn-primary" onClick={() => setOnboardingStep(3)} disabled={!apiUrl || !apiKey} style={{ display: "inline-flex", alignItems: "center", gap: "6px" }}>
                    <span>Lanjut</span>
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/>
                    </svg>
                  </button>
                </div>
                <p className="login-hint mt-3 text-center">*Dapatkan API Key dari menu Pengaturan API pada admin web.</p>
              </div>
            )}

            {/* ── Step 3: Activation & Encryption ── */}
            {onboardingStep === 3 && (
              <div className="onboarding-step-pane text-center">
                <div className="success-badge-avatar">
                  <svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/>
                  </svg>
                </div>

                <div className="pane-hero compact mt-2">
                  <h2>Aplikasi Siap Digunakan!</h2>
                  <p>Terminal POS kasir Anda telah dikonfigurasi dan database lokal telah terenkripsi secara otomatis.</p>
                </div>

                <div className="config-summary-table">
                  <div className="summary-row">
                    <span className="label">Server API</span>
                    <span className="value font-mono">{apiUrl}</span>
                  </div>
                  <div className="summary-row">
                    <span className="label">Enkripsi Database</span>
                    <span className="value tag-blue">SQLCipher AES-256</span>
                  </div>
                  <div className="summary-row">
                    <span className="label">Status Koneksi</span>
                    <span className="value tag-emerald">✓ Terverifikasi</span>
                  </div>
                </div>

                <div className="pane-actions-row mt-4">
                  <button className="btn-secondary" onClick={() => setOnboardingStep(2)}>
                    ← Ubah
                  </button>

                  <button className="btn-primary" style={{ flex: 1, padding: "12px", fontSize: "14px", display: "inline-flex", alignItems: "center", justifyContent: "center", gap: "8px" }} onClick={handleOnboardingFinish}>
                    <span>Selesaikan & Masuk Aplikasi</span>
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/>
                    </svg>
                  </button>
                </div>
              </div>
            )}
          </div>

          <div className="onboarding-brand-footer">
            <span>Partisipasi Sekolah v0.1.0 • Terminal POS Offline Kasir</span>
          </div>

        </div>
      </div>
    );
  }

  // ── Edit Activation (sudah pernah setup, ingin ganti server) ────────────────
  if (!isActivated || isEditingActivation) {
    return (
      <div className="login-screen-container">
        <div className="login-card">
          <div className="login-header">
            <span className="login-icon"><ActivationIcon /></span>
            <h2>Ubah Konfigurasi Server</h2>
            <p>Perbarui koneksi API terminal POS kasir lokal Anda.</p>
          </div>
          <form onSubmit={handleActivate} className="login-body mt-4">
            <div className="form-group">
              <label>API Server Online URL (Endpoint)</label>
              <input type="text" className="form-control" value={apiUrl} onChange={(e) => setApiUrl(e.target.value)} placeholder="https://partisipasi.sch.id" required />
            </div>
            <div className="form-group mt-3">
              <label>API Key Terminal</label>
              <input type="password" className="form-control" value={apiKey} onChange={(e) => setApiKey(e.target.value)} placeholder="Masukkan API Key terminal Anda..." required />
            </div>
            <button type="submit" className="btn-primary mt-4" style={{ width: "100%", display: "inline-flex", alignItems: "center", justifyContent: "center", gap: "8px" }} disabled={isTestingConnection}>
              <PlugIcon size={16} />
              {isTestingConnection ? "Mengetes Koneksi..." : "Simpan & Hubungkan"}
            </button>
            {isActivated && (
              <button type="button" className="btn-secondary mt-2" style={{ width: "100%" }} onClick={() => setIsEditingActivation(false)}>
                Batal
              </button>
            )}
          </form>
          <p className="login-hint mt-3">*Dapatkan API URL dan API Key terminal kasir Anda dari panel administrasi server online Partisipasi Sekolah (Pengaturan → API).</p>
        </div>
      </div>
    );
  }

  if (!session) {
    return (
      <div className="login-screen-container">
        <div className="login-card">
          <div className="login-header">
            <span className="login-icon"><KeyIcon /></span>
            <h2>Partisipasi Sekolah</h2>
            <p>Terminal Kasir POS & Tabungan Desktop Client</p>
            <div className="connection-badge mt-2" style={{ display: "inline-flex", alignItems: "center", gap: "6px", backgroundColor: "rgba(16, 185, 129, 0.08)", border: "1px solid rgba(16, 185, 129, 0.15)", padding: "4px 10px", borderRadius: "20px", fontSize: "11px", color: "var(--emerald)", fontWeight: "600", margin: "8px auto 0 auto" }}>
              <span className="dot online" style={{ width: "6px", height: "6px" }}></span>
              <span>Terhubung ke {apiUrl.replace(/^https?:\/\//, "")}</span>
            </div>
          </div>
          <div className="login-body mt-4">
            <button className="btn-primary" onClick={openLogin} style={{ width: "100%", display: "inline-flex", alignItems: "center", justifyContent: "center", gap: "8px" }}>
              <GlobeIcon /> Login via Browser SSO
            </button>
            <p className="login-hint mt-2 text-center">
              Membuka browser bawaan untuk masuk secara aman.
            </p>

            <div className="login-divider mt-4">
              <span>Atau masukkan data otorisasi secara manual</span>
            </div>

            <form onSubmit={handleManualLogin} className="mt-3">
              <div className="form-group">
                <label>Salin Link Otorisasi / Token</label>
                <input
                  type="text"
                  className="form-control"
                  value={manualToken}
                  onChange={(e) => setManualToken(e.target.value)}
                  placeholder="partisipasi-sekolah://auth?token=..."
                  required
                />
              </div>
              <button type="submit" className="btn-secondary mt-3" style={{ width: "100%" }}>
                Masuk ke Aplikasi
              </button>
            </form>
            <p className="login-hint mt-2" style={{ fontSize: "10px", color: "var(--text-muted)" }}>
              *Klik kanan tombol "Buka Aplikasi Desktop" di browser, pilih "Copy Link Address", lalu tempelkan di kolom atas.
            </p>

            <div className="text-center mt-4" style={{ textAlign: "center" }}>
              <button 
                type="button" 
                className="link-btn" 
                onClick={() => setIsEditingActivation(true)}
                style={{ fontSize: "11px", color: "var(--primary)", background: "none", border: "none", cursor: "pointer", textDecoration: "underline" }}
              >
                Ubah Pengaturan Koneksi API
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }



  const clientRole = session ? session.role : "";
  const roleDisplay = session ? (session.role === "admin" ? "ADMINISTRATOR" : session.role.toUpperCase()) : "";

  return (
    <div className="app-container">
      {/* Sidebar Navigation */}
      <aside className="sidebar">
        <div className="logo-container">
          <div className="logo-icon"><LogoIcon /></div>
          <div className="logo-text">
            <h2>POS Sekolah</h2>
            <span>Offline Client</span>
          </div>
        </div>

        <nav className="nav-menu">
          <button 
            className={`nav-item ${activeTab === "pos" ? "active" : ""}`}
            onClick={() => setActiveTab("pos")}
          >
            <MonitorIcon /> POS Kasir
          </button>
          
          {clientRole !== "kasir" && savingsEnabled && (
            <button 
              className={`nav-item ${activeTab === "savings" ? "active" : ""}`}
              onClick={() => setActiveTab("savings")}
            >
              <SavingsIcon /> Buku Tabungan
            </button>
          )}
          
          <button 
            className={`nav-item ${activeTab === "history" ? "active" : ""}`}
            onClick={() => setActiveTab("history")}
          >
            <HistoryIcon /> Riwayat Pembayaran
          </button>

          <button 
            className={`nav-item ${activeTab === "reports" ? "active" : ""}`}
            onClick={() => setActiveTab("reports")}
          >
            <ReportsIcon /> Laporan Kas
          </button>

          <button 
            className={`nav-item ${activeTab === "sync" ? "active" : ""}`}
            onClick={() => setActiveTab("sync")}
          >
            <SyncIcon size={18} /> Sync & Koneksi
          </button>
        </nav>

        <div className="status-panel">
          <div className="user-profile mt-2" style={{ marginBottom: "16px" }}>
            <div className="user-name"><UserIcon />{session.nama}</div>
            <div className="user-role">{roleDisplay}</div>
            <button className="logout-btn mt-2" onClick={handleLogout}>Keluar Sesi</button>
          </div>
          <div className="status-row">
            <span className={`dot ${isOnline ? "online" : "offline"}`}></span>
            <span>{isOnline ? "Terhubung (Online)" : "Mode Kasir Offline"}</span>
          </div>
          <div className="status-row mt-2">
            <span>Loket:</span>
            <input 
              value={loketId}
              onChange={(e) => setLoketId(e.target.value)} 
              className="loket-input"
            />
          </div>
        </div>
      </aside>

      {/* Main Content Area */}
      <main className="main-content">
        <header className="main-header">
          <div style={{ display: "flex", alignItems: "center", gap: "10px", flexGrow: 1, maxWidth: "650px" }}>
            <div className="search-bar" style={{ flexGrow: 1 }}>
              <SearchIcon size={16} className="search-bar-icon" />
              <input
                type="text"
                placeholder={studentStatus === "active" ? "Cari siswa aktif..." : "Cari alumni / siswa non-aktif..."}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
              {searchResults.length > 0 && (
                <div className="search-dropdown">
                  {searchResults.map((s) => (
                    <div 
                      key={s.id} 
                      className="search-item"
                      onClick={() => selectStudent(s)}
                    >
                      <div className="search-item-title">{s.nama}</div>
                      <div className="search-item-meta">
                        NISN: {s.nisn || "-"} | Kelas: {s.kelas || "Tidak ada kelas"}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div style={{
              display: "flex",
              border: "1px solid var(--border-color)",
              borderRadius: "8px",
              overflow: "hidden",
              backgroundColor: "var(--bg-tertiary)",
              height: "42px",
              padding: "2px",
              boxSizing: "border-box"
            }}>
              <button
                type="button"
                onClick={() => { setStudentStatus("active"); setSearchResults([]); }}
                style={{
                  padding: "0 14px",
                  fontSize: "12px",
                  fontWeight: "600",
                  cursor: "pointer",
                  border: "none",
                  borderRadius: "6px",
                  backgroundColor: studentStatus === "active" ? "var(--primary)" : "transparent",
                  color: studentStatus === "active" ? "#fff" : "var(--text-muted)",
                  transition: "all 0.2s ease",
                  display: "flex",
                  alignItems: "center",
                  height: "100%"
                }}
              >
                Siswa Aktif
              </button>
              <button
                type="button"
                onClick={() => { setStudentStatus("inactive"); setSearchResults([]); }}
                style={{
                  padding: "0 14px",
                  fontSize: "12px",
                  fontWeight: "600",
                  cursor: "pointer",
                  border: "none",
                  borderRadius: "6px",
                  backgroundColor: studentStatus === "inactive" ? "var(--primary)" : "transparent",
                  color: studentStatus === "inactive" ? "#fff" : "var(--text-muted)",
                  transition: "all 0.2s ease",
                  display: "flex",
                  alignItems: "center",
                  height: "100%"
                }}
              >
                Alumni
              </button>
            </div>
          </div>

          <div className="header-meta">
            <span className={`connection-badge ${isOnline ? "online" : "offline"}`} style={{
              display: "inline-flex",
              alignItems: "center",
              gap: "6px",
              padding: "4px 10px",
              borderRadius: "20px",
              fontSize: "12px",
              fontWeight: "600",
              backgroundColor: isOnline ? "rgba(16, 185, 129, 0.1)" : "rgba(239, 68, 68, 0.1)",
              color: isOnline ? "var(--emerald)" : "var(--rose)",
              border: isOnline ? "1px solid rgba(16, 185, 129, 0.2)" : "1px solid rgba(239, 68, 68, 0.2)"
            }}>
              <span style={{
                width: "8px",
                height: "8px",
                borderRadius: "50%",
                backgroundColor: isOnline ? "var(--emerald)" : "var(--rose)",
                display: "inline-block"
              }}></span>
              {isOnline ? "Online" : "Offline"}
            </span>
            <span className="year-badge">
              T.A Aktif: {academicYearName ?? (academicYearId ? "..." : "Memuat...")}
            </span>
            <button className="sync-shortcut" onClick={runSync} disabled={isSyncing} style={{ display: "inline-flex", alignItems: "center", gap: "6px" }}>
              <SyncIcon size={13} className={isSyncing ? "spinning" : ""} /> Sync
            </button>
          </div>
        </header>

        {/* Dashboard Content based on Active Tab */}
        <div className="content-body">
          {session.role === "admin" && (
            <div className="admin-alert-banner" style={{
              backgroundColor: "rgba(0, 43, 89, 0.04)",
              border: "1px solid var(--border-color)",
              padding: "12px 18px",
              borderRadius: "12px",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              fontSize: "13px",
              color: "var(--text-main)",
              gap: "12px",
              marginBottom: "10px"
            }}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <InfoIcon />
                <span>
                  Sebagai <strong>Administrator</strong>, Anda dapat mengonfigurasi pengaturan sistem, tahun pelajaran, kelas, & data user secara lengkap langsung di panel website.
                </span>
              </div>
              <button 
                className="btn-secondary" 
                onClick={() => openUrl(apiUrl)} 
                style={{ 
                  padding: "6px 12px", 
                  fontSize: "12px", 
                  width: "auto", 
                  whiteSpace: "nowrap",
                  borderColor: "var(--primary)",
                  color: "var(--primary)",
                  cursor: "pointer"
                }}
              >
                Buka Panel Website
              </button>
            </div>
          )}

          {/* TAB 1: POS / KASIR */}
          {activeTab === "pos" && (
            <div className="tab-pos-container">
              {/* Stat Cards at the top of POS dashboard */}
              <div className="summary-cards" style={{ marginBottom: "20px" }}>
                <div className="card tagihan" style={{ borderLeft: "4px solid var(--primary)" }}>
                  <span style={{ fontSize: "12px", color: "var(--text-muted)", fontWeight: 500 }}>Pemasukan Hari Ini</span>
                  <h4 style={{ color: "var(--primary)", fontSize: "20px", fontWeight: 700, margin: "4px 0 0 0" }}>
                    Rp {todayStats.today_total.toLocaleString("id-ID")}
                  </h4>
                </div>
                <div className="card lunas" style={{ borderLeft: "4px solid var(--emerald)" }}>
                  <span style={{ fontSize: "12px", color: "var(--text-muted)", fontWeight: 500 }}>Pemasukan Bulan Ini</span>
                  <h4 style={{ color: "var(--emerald)", fontSize: "20px", fontWeight: 700, margin: "4px 0 0 0" }}>
                    Rp {todayStats.monthly_total.toLocaleString("id-ID")}
                  </h4>
                </div>
                <div className="card tunggakan" style={{ borderLeft: "4px solid var(--amber)" }}>
                  <span style={{ fontSize: "12px", color: "var(--text-muted)", fontWeight: 500 }}>Transaksi Hari Ini</span>
                  <h4 style={{ color: "var(--amber)", fontSize: "20px", fontWeight: 700, margin: "4px 0 0 0" }}>
                    {todayStats.today_count.toLocaleString("id-ID")} Transaksi
                  </h4>
                </div>
              </div>

              <div className="grid-pos">
                <div className="pos-left">
                  {selectedStudent && studentDetails && !loadingDetails ? (
                    <>
                      {/* Active Student Bar */}
                      <div className="student-profile-bar" style={{ marginBottom: "16px" }}>
                        <div className="profile-avatar"><StudentIcon size={28} /></div>
                        <div className="profile-info">
                          <h3>{selectedStudent.nama}</h3>
                          <p>NISN: {selectedStudent.nisn || "-"} | NIS: {selectedStudent.nis || "-"} | Kelas: {selectedStudent.kelas || "-"}</p>
                        </div>
                        <button className="clear-student-btn" onClick={() => { setSelectedStudent(null); setStudentDetails(null); }} style={{ display: "inline-flex", alignItems: "center", gap: "6px" }}>
                          <CloseIcon /> Ganti Siswa
                        </button>
                      </div>

                      {/* Warning for Unpaid Previous Years */}
                      {studentDetails.unpaid_years && studentDetails.unpaid_years.length > 0 && (
                        <div className="previous-year-warning" style={{
                          backgroundColor: "rgba(186, 26, 26, 0.08)",
                          borderLeft: "4px solid var(--rose)",
                          padding: "16px",
                          borderRadius: "12px",
                          marginBottom: "16px",
                          color: "var(--rose)",
                          fontSize: "13px",
                          lineHeight: "1.5"
                        }}>
                          <div style={{ fontWeight: 800, fontSize: "14px", marginBottom: "4px", display: "flex", alignItems: "center", gap: "6px" }}>
                            <InfoIcon /> Perhatian!
                          </div>
                          Ada pembayaran yang belum diselesaikan pada tahun pelajaran: <strong>{studentDetails.unpaid_years.join(", ")}</strong>. Silakan konfirmasi ke orang tua/murid.
                        </div>
                      )}

                      {/* Bills grouped by Academic Year (Accordion Style) */}
                      {(() => {
                        const groups: { [key: string]: { name: string; items: PaymentAssignment[] } } = {};
                        studentDetails.assignments.forEach((a) => {
                          const yearId = a.academic_year_id || "none";
                          if (!groups[yearId]) {
                            groups[yearId] = {
                              name: a.academic_year_name || "Tidak Diketahui",
                              items: []
                            };
                          }
                          groups[yearId].items.push(a);
                        });

                        const sortedYearIds = Object.keys(groups).sort((a, b) => {
                          const nameA = groups[a].name;
                          const nameB = groups[b].name;
                          return nameB.localeCompare(nameA); // terbaru lebih dulu
                        });

                        if (sortedYearIds.length === 0) {
                          return (
                            <div className="table-container" style={{ textAlign: "center", padding: "24px" }}>
                              <p className="text-muted">Tidak ada tagihan yang di-assign untuk siswa ini.</p>
                            </div>
                          );
                        }

                        return sortedYearIds.map((yearId) => {
                          const group = groups[yearId];
                          const isExpanded = expandedYears[yearId] ?? false;
                          const isActiveYear = yearId === studentDetails.active_year_id;

                          return (
                            <div key={yearId} className="year-group-container" style={{ marginBottom: "16px", border: "1px solid var(--border-color)", borderRadius: "12px", overflow: "hidden", backgroundColor: "var(--bg-tertiary)" }}>
                              <div 
                                className="year-group-header" 
                                onClick={() => setExpandedYears({ ...expandedYears, [yearId]: !isExpanded })}
                                style={{ 
                                  display: "flex", 
                                  alignItems: "center", 
                                  justifyContent: "space-between", 
                                  padding: "14px 18px", 
                                  backgroundColor: "var(--bg-secondary)", 
                                  cursor: "pointer",
                                  fontWeight: "700",
                                  fontSize: "14px",
                                  color: "var(--primary)",
                                  userSelect: "none"
                                }}
                              >
                                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                                  <ClockIcon />
                                  <span>T.A {group.name}</span>
                                  {isActiveYear && (
                                    <span style={{ fontSize: "10px", backgroundColor: "var(--primary-light)", color: "var(--on-primary-light)", padding: "2px 8px", borderRadius: "20px", fontWeight: "600" }}>Aktif</span>
                                  )}
                                </div>
                                <span style={{ transition: "transform 0.2s ease", transform: isExpanded ? "rotate(0deg)" : "rotate(-90deg)" }}>
                                  ▼
                                </span>
                              </div>

                              {isExpanded && (
                                <div className="table-container" style={{ border: "none", borderRadius: "0", padding: "0" }}>
                                  <table className="custom-table" style={{ width: "100%" }}>
                                    <thead>
                                      <tr>
                                        <th>Jenis {terms.pembayaran}</th>
                                        <th>Bulan</th>
                                        {showNominal.portal && <th>Nominal</th>}
                                        {showNominal.portal && <th>Potongan</th>}
                                        {showNominal.portal && <th>Terbayar</th>}
                                        {showNominal.portal && <th>Sisa</th>}
                                        <th>Status</th>
                                        <th>Bayar Kasir</th>
                                      </tr>
                                    </thead>
                                    <tbody>
                                      {group.items.map((a) => (
                                        <tr key={a.id}>
                                          <td><strong>{a.payment_type_name}</strong></td>
                                          <td>{a.month_name || "-"}</td>
                                          {showNominal.portal && <td>Rp {a.amount.toLocaleString("id-ID")}</td>}
                                          {showNominal.portal && <td className="text-blue">Rp {a.relief_amount.toLocaleString("id-ID")}</td>}
                                          {showNominal.portal && <td className="text-emerald">Rp {a.paid_amount.toLocaleString("id-ID")}</td>}
                                          {showNominal.portal && <td><strong>Rp {a.remaining_amount.toLocaleString("id-ID")}</strong></td>}
                                          <td>
                                            <span className={`status-badge ${a.status}`}>
                                              {a.status.replace("_", " ")}
                                            </span>
                                          </td>
                                          <td>
                                            {a.remaining_amount > 0 ? (
                                              <input
                                                type="number"
                                                min="0"
                                                max={a.remaining_amount}
                                                placeholder="0"
                                                className="table-input"
                                                value={cart.find(item => item.assignmentId === a.id)?.payAmount || ""}
                                                onChange={(e) => handleCartChange(a, parseFloat(e.target.value) || 0)}
                                              />
                                            ) : (
                                              <span className="text-muted">Lunas</span>
                                            )}
                                          </td>
                                        </tr>
                                      ))}
                                    </tbody>
                                  </table>
                                </div>
                              )}
                            </div>
                          );
                        });
                      })()}
                    </>
                  ) : (
                    <div className="welcome-banner" style={{ padding: "64px 32px" }}>
                      <span style={{ display: "inline-block", color: "rgba(255,255,255,0.7)", marginBottom: "12px" }}>
                        <SearchIcon size={48} />
                      </span>
                      <h2 style={{ fontSize: "20px", fontWeight: "700", marginTop: "12px" }}>Cari Data Siswa</h2>
                      <p style={{ color: "rgba(255,255,255,0.8)" }}>Silakan ketik nama, NISN, atau kelas pada kotak pencarian di bagian atas untuk memulai transaksi pembayaran kasir.</p>
                    </div>
                  )}
                </div>

                <div className="pos-right">
                  {selectedStudent && studentDetails && !loadingDetails ? (
                    <>
                      {/* POS Cart Panel */}
                      <div className="cart-card">
                        <h3>Keranjang Pembayaran</h3>
                        
                        {paymentSuccessMsg && (
                          <div className="alert-success" style={{ padding: "10px", backgroundColor: "rgba(16, 185, 129, 0.1)", border: "1px solid var(--emerald)", borderRadius: "6px", color: "var(--emerald)", fontSize: "13px", marginBottom: "12px" }}>
                            {paymentSuccessMsg}
                          </div>
                        )}

                        {cart.length === 0 ? (
                          <div className="empty-cart">
                            <p>Keranjang kosong. Masukkan nominal pada daftar tagihan di sebelah kiri.</p>
                          </div>
                        ) : (
                          <form onSubmit={processPayment}>
                            <div className="cart-items">
                              {cart.map((item) => (
                                <div key={item.assignmentId} className="cart-item">
                                  <div className="cart-item-name">{item.name}</div>
                                  <div className="cart-item-val">
                                    Rp {item.payAmount.toLocaleString("id-ID")}
                                  </div>
                                </div>
                              ))}
                            </div>

                            <div className="cart-total-row">
                              <span>Total Pembayaran:</span>
                              <strong>
                                Rp {cart.reduce((acc, item) => acc + item.payAmount, 0).toLocaleString("id-ID")}
                              </strong>
                            </div>

                            <div className="form-group mt-3">
                              <label>Metode Pembayaran</label>
                              <select 
                                value={paymentMethodName}
                                onChange={(e) => setPaymentMethodName(e.target.value)}
                                className="form-control"
                              >
                                <option value="Tunai">Tunai</option>
                                <option value="Transfer Bank">Transfer Bank</option>
                                <option value="Potong Tabungan">Potong Tabungan</option>
                              </select>
                            </div>

                            <div className="form-group mt-2">
                              <label>Catatan / Keterangan</label>
                              <textarea
                                value={paymentNote}
                                onChange={(e) => setPaymentNote(e.target.value)}
                                placeholder="Contoh: Titipan orang tua..."
                                className="form-control"
                              />
                            </div>

                            <button 
                              type="submit" 
                              className="btn-primary mt-3"
                              disabled={isProcessingPayment}
                              style={{ width: "100%" }}
                            >
                              {isProcessingPayment ? "Memproses..." : "Proses & Cetak Kwitansi"}
                            </button>
                          </form>
                        )}
                      </div>

                      {/* Student's Payment History */}
                      <div className="cart-card mt-3">
                        <h3>Riwayat Bayar Siswa</h3>
                        <div className="history-list">
                          {studentDetails.payments.length === 0 ? (
                            <p className="text-muted" style={{ fontSize: "13px" }}>Belum ada riwayat pembayaran.</p>
                          ) : (
                            studentDetails.payments.map((p) => (
                              <div key={p.id} className="history-item" style={{ display: "flex", justifyContent: "space-between", fontSize: "13px", borderBottom: "1px solid var(--border-color)", padding: "8px 0" }}>
                                <div>
                                  <strong>{p.transaction_code}</strong>
                                  <div className="text-muted text-xs">{p.created_at}</div>
                                </div>
                                <div style={{ fontWeight: 600 }}>
                                  Rp {p.total_amount.toLocaleString("id-ID")}
                                </div>
                              </div>
                            ))
                          )}
                        </div>
                      </div>
                    </>
                  ) : (
                    <div className="cart-card">
                      <h3>Petunjuk Pintasan</h3>
                      <div style={{ fontSize: "13px", lineHeight: "1.6", color: "var(--text-muted)" }}>
                        <p style={{ display: "flex", alignItems: "flex-start", gap: "6px" }}>
                          <InfoIcon /> <span><strong>Mode Kasir Offline</strong> dirancang untuk mencatat transaksi pembayaran sekolah meskipun koneksi internet terputus.</span>
                        </p>
                        <p className="mt-2">1. Lakukan pencarian siswa di header atas.</p>
                        <p>2. Input jumlah bayar kasir pada tagihan.</p>
                        <p>3. Pilih metode pembayaran, masukkan catatan.</p>
                        <p>4. Klik proses untuk mencetak struk secara lokal.</p>
                        <p>5. Lakukan sinkronisasi data secara berkala ke database online.</p>
                      </div>
                    </div>
                  )}
                </div>
              </div>

              {/* Today's Transactions History List at the Bottom */}
              <div className="table-container mt-4" style={{ border: "1px solid var(--border-color)" }}>
                <h3 style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "16px", fontWeight: "600" }}>
                  <ClockIcon /> Riwayat Pembayaran Kasir Hari Ini (Lokal)
                </h3>
                {recentTransactions.length === 0 ? (
                  <div className="empty-cart" style={{ padding: "24px", textAlign: "center", color: "var(--text-muted)" }}>
                    Belum ada transaksi pembayaran lokal yang diproses hari ini.
                  </div>
                ) : (
                  <table className="custom-table" style={{ width: "100%" }}>
                    <thead>
                      <tr>
                        <th>Ref ID</th>
                        <th>Waktu</th>
                        <th>Nama Siswa</th>
                        <th>Kelas</th>
                        <th>Jumlah Bayar</th>
                        <th>Status Sync</th>
                        <th style={{ width: "80px", textAlign: "center" }}>Aksi</th>
                      </tr>
                    </thead>
                    <tbody>
                      {recentTransactions.map((tx) => (
                        <tr key={tx.id}>
                          <td><span style={{ fontFamily: "monospace", fontWeight: "bold", color: "var(--primary)" }}>{tx.transaction_code}</span></td>
                          <td>{new Date(tx.created_at).toLocaleTimeString("id-ID", { hour: "2-digit", minute: "2-digit" })}</td>
                          <td><strong>{tx.student_name}</strong></td>
                          <td>{tx.student_class || "-"}</td>
                          <td style={{ fontWeight: 600, color: "var(--emerald)" }}>Rp {tx.total_amount.toLocaleString("id-ID")}</td>
                          <td>
                            <span className={`status-badge ${tx.sync_status}`}>
                              {tx.sync_status === "synced" ? "Tersinkron" : "Lokal"}
                            </span>
                          </td>
                          <td style={{ textAlign: "center" }}>
                            <button
                              onClick={() => triggerReceiptPrint(tx.transaction_code)}
                              title="Cetak Kwitansi"
                              style={{
                                background: "none",
                                border: "none",
                                color: "var(--primary)",
                                cursor: "pointer",
                                padding: "4px",
                                display: "inline-flex",
                                alignItems: "center"
                              }}
                            >
                              <PrintIcon size={16} />
                            </button>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
            </div>
          )}

          {/* TAB 2: TABUNGAN MURID */}
          {activeTab === "savings" && (
            <div className="tab-savings-container">
              {selectedStudent && studentDetails && !loadingDetails ? (
                <div className="grid-pos">
                  <div className="pos-left">
                    <div className="cart-card">
                      <h3>Buku Tabungan Siswa</h3>
                      <div className="savings-balance-hero" style={{ backgroundColor: "var(--bg-secondary)", border: "1px solid var(--border-color)", padding: "16px", borderRadius: "12px", display: "flex", flexDirection: "column", gap: "4px" }}>
                        <span style={{ fontSize: "13px", color: "var(--text-muted)" }}>Saldo Tabungan Aktif</span>
                        <h2 style={{ fontSize: "28px", color: "var(--emerald)", fontWeight: "800" }}>
                          Rp {studentDetails.savings_balance.toLocaleString("id-ID")}
                        </h2>
                      </div>

                      <form onSubmit={handleSavingsTransaction} className="mt-4">
                        <div className="form-group">
                          <label>Tipe Transaksi</label>
                          <div className="radio-group" style={{ display: "flex", gap: "10px", marginTop: "6px" }}>
                            <button
                              type="button"
                              className={`btn ${savingsType === "deposit" ? "btn-primary" : "btn-secondary"}`}
                              style={{ flex: 1, padding: "10px", display: "inline-flex", alignItems: "center", justifyContent: "center" }}
                              onClick={() => setSavingsType("deposit")}
                            >
                              <DepositIcon /> Setor Dana
                            </button>
                            <button
                              type="button"
                              className={`btn ${savingsType === "withdraw" ? "btn-primary" : "btn-secondary"}`}
                              style={{ flex: 1, padding: "10px", display: "inline-flex", alignItems: "center", justifyContent: "center" }}
                              onClick={() => setSavingsType("withdraw")}
                            >
                              <WithdrawIcon /> Tarik Dana
                            </button>
                          </div>
                        </div>

                        <div className="form-group mt-3">
                          <label>Nominal Transaksi (Rp)</label>
                          <input
                            type="number"
                            min="1000"
                            placeholder="Contoh: 50000"
                            className="form-control"
                            value={savingsAmount || ""}
                            onChange={(e) => setSavingsAmount(parseFloat(e.target.value) || 0)}
                            required
                          />
                        </div>

                        <div className="form-group mt-3">
                          <label>Catatan / Log</label>
                          <textarea
                            className="form-control"
                            placeholder="Keterangan transaksi tabungan..."
                            value={savingsNote}
                            onChange={(e) => setSavingsNote(e.target.value)}
                          />
                        </div>

                        <button 
                          type="submit" 
                          className="btn-primary mt-4"
                          disabled={isProcessingSavings}
                          style={{ width: "100%" }}
                        >
                          {isProcessingSavings ? "Memproses..." : "Simpan Transaksi Tabungan"}
                        </button>
                      </form>
                    </div>
                  </div>

                  <div className="pos-right">
                    <div className="cart-card">
                      <h3>Deskripsi Aturan Tabungan</h3>
                      <p className="text-muted text-sm" style={{ fontSize: "13px", lineHeight: "1.6" }}>
                        Penyetoran dan penarikan tabungan tercatat di database lokal secara offline. 
                        Jurnal akuntansi keuangan (Debet/Kredit) akan dibuat otomatis saat transaksi disetujui.
                      </p>
                      <ul className="text-muted text-sm mt-2" style={{ fontSize: "13px", paddingLeft: "20px", lineHeight: "1.6" }}>
                        <li>Pastikan jumlah penarikan tidak melebihi saldo tabungan aktif siswa.</li>
                        <li>Dana tabungan yang tersimpan dapat disinkronkan ke server pusat saat menekan tombol Sync.</li>
                      </ul>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="welcome-banner" style={{ padding: "64px 32px" }}>
                  <span style={{ display: "inline-block", color: "rgba(255,255,255,0.7)", marginBottom: "12px" }}>
                    <SavingsIcon />
                  </span>
                  <h2 style={{ fontSize: "20px", fontWeight: "700", marginTop: "12px" }}>Buku Tabungan Murid</h2>
                  <p style={{ color: "rgba(255,255,255,0.8)" }}>Silakan cari dan pilih siswa pada kotak pencarian di atas terlebih dahulu untuk memproses setor/tarik tabungan.</p>
                </div>
              )}
            </div>
          )}

          {/* TAB 3: SYNC ENGINE SETTINGS */}
          {activeTab === "sync" && (
            <div className="sync-layout">
              <div className="sync-settings-card">
                <h3>Konfigurasi Integrasi & Sync Engine</h3>
                <form onSubmit={saveConfig} className="mt-3">
                  <div className="form-group">
                    <label>URL API Server Online</label>
                    <input
                      type="url"
                      className="form-control"
                      value={apiUrl}
                      onChange={(e) => setApiUrl(e.target.value)}
                      placeholder="http://nama-sekolah.sch.id"
                      required
                    />
                  </div>

                  <div className="form-group mt-3">
                    <label>X-API-Key</label>
                    <input
                      type="password"
                      className="form-control"
                      value={apiKey}
                      onChange={(e) => setApiKey(e.target.value)}
                      placeholder="psk_live_..."
                      required
                    />
                  </div>

                  <div className="button-group mt-4" style={{ display: "flex", gap: "10px" }}>
                    <button type="submit" className="btn-secondary" style={{ flex: 1 }}>
                      Simpan Konfigurasi
                    </button>
                    <button 
                      type="button" 
                      className="btn-primary" 
                      onClick={runSync} 
                      disabled={isSyncing}
                      style={{ flex: 1.2, display: "inline-flex", alignItems: "center", justifyContent: "center", gap: "8px" }}
                    >
                      <SyncIcon size={16} className={isSyncing ? "spinning" : ""} />
                      {isSyncing ? "Menyinkronkan..." : "Sinkronisasikan Sekarang"}
                    </button>
                  </div>
                </form>
              </div>

              <div className="sync-status-card">
                <h3>Status Sync Terakhir</h3>
                <div className="sync-meta-list">
                  <div className="sync-meta-item">
                    <span>Status Koneksi:</span>
                    <strong className="text-emerald">Terhubung (Online)</strong>
                  </div>
                  <div className="sync-meta-item">
                    <span>Waktu Sync Terakhir:</span>
                    <span>{lastSyncTime}</span>
                  </div>
                </div>

                {syncStatusMsg && (
                  <div className="sync-console-log mt-3">
                    <strong>Console Log Sinkronisasi:</strong>
                    <pre>{syncStatusMsg}</pre>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* TAB 4: RIWAYAT PEMBAYARAN GLOBAL */}
          {activeTab === "history" && (
            <div className="tab-history-container mt-4">
              <div className="table-container" style={{ border: "1px solid var(--border-color)", padding: "20px", borderRadius: "12px", backgroundColor: "var(--bg-secondary)" }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "18px", gap: "16px", flexWrap: "wrap" }}>
                  <h3 style={{ margin: 0, display: "flex", alignItems: "center", gap: "8px", fontSize: "16px", fontWeight: "600" }}>
                    <HistoryIcon /> Semua Riwayat Pembayaran (Lokal Offline)
                  </h3>
                  <div style={{ display: "flex", alignItems: "center", gap: "10px", width: "100%", maxWidth: "350px" }}>
                    <div className="search-bar" style={{ width: "100%", margin: 0 }}>
                      <SearchIcon size={16} className="search-bar-icon" />
                      <input
                        type="text"
                        placeholder="Cari transaksi / nama siswa / kelas..."
                        value={historySearchQuery}
                        onChange={(e) => setHistorySearchQuery(e.target.value)}
                      />
                    </div>
                  </div>
                </div>

                {historyList.length === 0 ? (
                  <div className="empty-cart" style={{ padding: "48px 24px", textAlign: "center", color: "var(--text-muted)" }}>
                    Tidak ditemukan data riwayat transaksi pembayaran.
                  </div>
                ) : (
                  <>
                    <table className="custom-table" style={{ width: "100%" }}>
                      <thead>
                        <tr>
                          <th>Ref ID</th>
                          <th>Waktu & Tanggal</th>
                          <th>Nama Siswa</th>
                          <th>Kelas</th>
                          <th>Total Bayar</th>
                          <th>Metode</th>
                          <th>Status Sync</th>
                          <th style={{ width: "80px", textAlign: "center" }}>Aksi</th>
                        </tr>
                      </thead>
                      <tbody>
                        {historyList.map((tx) => (
                          <tr key={tx.id}>
                            <td>
                              <span style={{ fontFamily: "monospace", fontWeight: "bold", color: "var(--primary)" }}>
                                {tx.transaction_code}
                              </span>
                            </td>
                            <td>
                              {new Date(tx.created_at).toLocaleString("id-ID", {
                                day: "2-digit",
                                month: "short",
                                year: "numeric",
                                hour: "2-digit",
                                minute: "2-digit"
                              })}
                            </td>
                            <td><strong>{tx.student_name}</strong></td>
                            <td>{tx.student_class || "-"}</td>
                            <td style={{ fontWeight: 600, color: "var(--emerald)" }}>
                              Rp {tx.total_amount.toLocaleString("id-ID")}
                            </td>
                            <td><span style={{ fontSize: "12px", color: "var(--text-muted)" }}>{tx.payment_method_name}</span></td>
                            <td>
                              <span className={`status-badge ${tx.sync_status}`}>
                                {tx.sync_status === "synced" ? "Tersinkron" : "Lokal"}
                              </span>
                            </td>
                            <td style={{ textAlign: "center" }}>
                              <button
                                onClick={() => triggerReceiptPrint(tx.transaction_code)}
                                title="Cetak Kwitansi"
                                style={{
                                  background: "none",
                                  border: "none",
                                  color: "var(--primary)",
                                  cursor: "pointer",
                                  padding: "4px",
                                  display: "inline-flex",
                                  alignItems: "center"
                                }}
                              >
                                <PrintIcon size={16} />
                              </button>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>

                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: "20px" }}>
                      <button
                        className="btn-secondary"
                        disabled={historyPage <= 1}
                        onClick={() => loadHistory(historyPage - 1, historySearchQuery, false)}
                        style={{ padding: "6px 12px", fontSize: "13px" }}
                      >
                        ← Halaman Sebelumnya
                      </button>
                      <span style={{ fontSize: "13px", fontWeight: "600", color: "var(--text-muted)" }}>
                        Halaman {historyPage}
                      </span>
                      <button
                        className="btn-secondary"
                        disabled={historyList.length < 30}
                        onClick={() => loadHistory(historyPage + 1, historySearchQuery, false)}
                        style={{ padding: "6px 12px", fontSize: "13px" }}
                      >
                        Halaman Selanjutnya →
                      </button>
                    </div>
                  </>
                )}
              </div>
            </div>
          )}

          {/* TAB 5: LAPORAN KAS LOKAL */}
          {activeTab === "reports" && (
            <div className="tab-reports-container mt-4" style={{ display: "flex", flexDirection: "column", gap: "20px" }}>
              {/* Stat Cards */}
              <div className="summary-cards">
                <div className="card tagihan" style={{ borderLeft: "4px solid var(--primary)" }}>
                  <span style={{ fontSize: "12px", color: "var(--text-muted)", fontWeight: 500 }}>Total Pemasukan Kasir</span>
                  <h4 style={{ color: "var(--primary)", fontSize: "22px", fontWeight: 700, margin: "4px 0 0 0" }}>
                    Rp {reportsData?.total_collected?.toLocaleString("id-ID") || 0}
                  </h4>
                </div>
                <div className="card lunas" style={{ borderLeft: "4px solid var(--emerald)" }}>
                  <span style={{ fontSize: "12px", color: "var(--text-muted)", fontWeight: 500 }}>Total Transaksi</span>
                  <h4 style={{ color: "var(--emerald)", fontSize: "22px", fontWeight: 700, margin: "4px 0 0 0" }}>
                    {reportsData?.total_txs || 0} Transaksi
                  </h4>
                </div>
                <div className="card tunggakan" style={{ borderLeft: "4px solid var(--rose)" }}>
                  <span style={{ fontSize: "12px", color: "var(--text-muted)", fontWeight: 500 }}>Estimasi Sisa Piutang (DB)</span>
                  <h4 style={{ color: "var(--rose)", fontSize: "22px", fontWeight: 700, margin: "4px 0 0 0" }}>
                    Rp {reportsData?.remaining_receivables?.toLocaleString("id-ID") || 0}
                  </h4>
                </div>
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "20px", flexWrap: "wrap" }}>
                {/* Method Breakdown */}
                <div className="table-container" style={{ border: "1px solid var(--border-color)", padding: "20px", borderRadius: "12px", backgroundColor: "var(--bg-secondary)" }}>
                  <h3 style={{ margin: "0 0 16px 0", fontSize: "15px", fontWeight: "600", color: "var(--text-main)" }}>
                    Penerimaan per Metode Pembayaran
                  </h3>
                  {!reportsData?.methods || reportsData.methods.length === 0 ? (
                    <div style={{ padding: "20px", textAlign: "center", color: "var(--text-muted)" }}>Belum ada data transaksi.</div>
                  ) : (
                    <table className="custom-table" style={{ width: "100%" }}>
                      <thead>
                        <tr>
                          <th>Metode</th>
                          <th style={{ textAlign: "center" }}>Jumlah Transaksi</th>
                          <th style={{ textAlign: "right" }}>Total Penerimaan</th>
                        </tr>
                      </thead>
                      <tbody>
                        {reportsData.methods.map((m: any, idx: number) => (
                          <tr key={idx}>
                            <td><strong>{m.method}</strong></td>
                            <td style={{ textAlign: "center" }}>{m.count}</td>
                            <td style={{ textAlign: "right", fontWeight: 600, color: "var(--emerald)" }}>Rp {m.total.toLocaleString("id-ID")}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>

                {/* Pos Breakdown */}
                <div className="table-container" style={{ border: "1px solid var(--border-color)", padding: "20px", borderRadius: "12px", backgroundColor: "var(--bg-secondary)" }}>
                  <h3 style={{ margin: "0 0 16px 0", fontSize: "15px", fontWeight: "600", color: "var(--text-main)" }}>
                    Penerimaan per Pos / Jenis Tagihan
                  </h3>
                  {!reportsData?.pos_breakdown || reportsData.pos_breakdown.length === 0 ? (
                    <div style={{ padding: "20px", textAlign: "center", color: "var(--text-muted)" }}>Belum ada rincian pos.</div>
                  ) : (
                    <table className="custom-table" style={{ width: "100%" }}>
                      <thead>
                        <tr>
                          <th>Pos Pembayaran</th>
                          <th style={{ textAlign: "right" }}>Total Penerimaan</th>
                        </tr>
                      </thead>
                      <tbody>
                        {reportsData.pos_breakdown.map((p: any, idx: number) => (
                          <tr key={idx}>
                            <td><strong>{p.pos}</strong></td>
                            <td style={{ textAlign: "right", fontWeight: 600, color: "var(--primary)" }}>Rp {p.total.toLocaleString("id-ID")}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>
              </div>

              {/* External Reports Redirect link */}
              <div style={{
                border: "1px solid var(--border-color)",
                padding: "20px",
                borderRadius: "12px",
                backgroundColor: "rgba(0, 43, 89, 0.03)",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                flexWrap: "wrap",
                gap: "16px"
              }}>
                <div>
                  <h4 style={{ margin: 0, fontSize: "15px", fontWeight: 600 }}>Butuh Laporan Lengkap & Grafik?</h4>
                  <p style={{ margin: "4px 0 0 0", fontSize: "13px", color: "var(--text-muted)" }}>
                    Anda dapat mengakses Dashboard Laporan Lintas Tahun Pelajaran, Ekspor PDF/Excel, serta Analisis Piutang secara lengkap di panel admin website.
                  </p>
                </div>
                <button
                  onClick={() => openUrl(apiUrl + "/reports")}
                  className="btn-primary"
                  style={{ display: "inline-flex", alignItems: "center", gap: "6px" }}
                >
                  <GlobeIcon /> Buka Laporan Website
                </button>
              </div>
            </div>
          )}
        </div>
      </main>

      {showReceiptModal && printReceiptData && (
        <div className="receipt-modal-overlay">
          <div className="receipt-modal-container">
            <div className="receipt-modal-header">
              <h3>{terms.kwitansi}</h3>
              <button 
                onClick={() => { setShowReceiptModal(false); setPrintReceiptData(null); }}
                style={{ background: "none", border: "none", fontSize: "20px", cursor: "pointer", color: "#666" }}
              >
                ✕
              </button>
            </div>
            
            <div className="receipt-modal-body">
              <div className="receipt-printable" id="receipt-print-area">
                {/* Header */}
                <div className="receipt-header-print">
                  {schoolSettings.logo_left && (
                    <img src={schoolSettings.logo_left} alt="Logo" className="receipt-logo-print" />
                  )}
                  <div className="receipt-school-details">
                    {schoolSettings.committee_name && (
                      <h2>{schoolSettings.committee_name}</h2>
                    )}
                    <h2>{schoolSettings.school_name || "NAMA SEKOLAH"}</h2>
                    <p>{schoolSettings.school_address || "ALAMAT SEKOLAH"}</p>
                  </div>
                  {schoolSettings.logo_right && (
                    <img src={schoolSettings.logo_right} alt="Logo" className="receipt-logo-print" />
                  )}
                </div>

                {/* Title */}
                <div className="receipt-title-print">{terms.kwitansi} {terms.pembayaran}</div>

                {/* Metadata */}
                <table className="receipt-info-table">
                  <tbody>
                    <tr>
                      <td className="label-cell">No. Transaksi</td>
                      <td>: {printReceiptData.transaction_code}</td>
                      <td className="label-cell">Tanggal</td>
                      <td>: {new Date(printReceiptData.created_at).toLocaleDateString("id-ID", { day: "2-digit", month: "2-digit", year: "numeric", hour: "2-digit", minute: "2-digit" })}</td>
                    </tr>
                    <tr>
                      <td className="label-cell">Nama {terms.siswa}</td>
                      <td>: {printReceiptData.student_nama}</td>
                      <td className="label-cell">NIS</td>
                      <td>: {printReceiptData.student_nis || "-"}</td>
                    </tr>
                    <tr>
                      <td className="label-cell">Kelas</td>
                      <td>: {printReceiptData.student_kelas || "-"}</td>
                      <td className="label-cell">Metode</td>
                      <td>: {printReceiptData.payment_method_name || "Tunai"}</td>
                    </tr>
                  </tbody>
                </table>

                {/* Items Table */}
                <table className="receipt-items-table">
                  <thead>
                    <tr>
                      <th style={{ width: "40px" }}>No</th>
                      <th>Keterangan</th>
                      <th className="amount-cell">{terms.tagihan}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {printReceiptData.items.map((item: any, idx: number) => (
                      <tr key={idx}>
                        <td>{idx + 1}</td>
                        <td>
                          {item.payment_type_name}
                          {item.month_name ? ` — ${item.month_name}` : ""}
                          {item.academic_year_name ? ` (T.A ${item.academic_year_name})` : ""}
                        </td>
                        <td className="amount-cell">{showNominal.receipt ? `Rp ${item.amount.toLocaleString("id-ID")}` : "—"}</td>
                      </tr>
                    ))}
                  </tbody>
                  <tfoot>
                    <tr className="receipt-total-row">
                      <td colSpan={2} style={{ textAlign: "right", fontWeight: "700" }}>Total</td>
                      <td className="amount-cell" style={{ fontWeight: "700" }}>
                        {showNominal.receipt ? `Rp ${printReceiptData.total_amount.toLocaleString("id-ID")}` : "—"}
                      </td>
                    </tr>
                  </tfoot>
                </table>

                {printReceiptData.notes && (
                  <p style={{ fontSize: "11px", marginBottom: "16px" }}>
                    <strong>Catatan:</strong> {printReceiptData.notes}
                  </p>
                )}

                {/* Footer / Signature */}
                <div className="receipt-sign-section">
                  <div className="receipt-sign-box">
                    <p style={{ margin: 0 }}>Mojokerto, {new Date(printReceiptData.created_at).toLocaleDateString("id-ID", { day: "2-digit", month: "long", year: "numeric" })}</p>
                    <p style={{ margin: "2px 0 0 0", fontWeight: "700" }}>Petugas Penerima,</p>
                    
                    {schoolSettings.stamp_image && (
                      <img src={schoolSettings.stamp_image} alt="Stempel" className="receipt-stamp-img" />
                    )}

                    <div className="receipt-sign-name">
                      {session ? session.nama : "Petugas"}
                    </div>
                  </div>
                </div>

                <p style={{ textAlign: "center", fontSize: "9px", color: "#888", marginTop: "24px" }}>
                  Dicetak pada {new Date().toLocaleString("id-ID")} — {schoolSettings.school_name || "Sistem Kasir"}
                </p>
              </div>
            </div>
            
            <div className="receipt-modal-footer">
              <button 
                className="btn btn-outline" 
                onClick={() => { setShowReceiptModal(false); setPrintReceiptData(null); }}
                style={{ padding: "8px 16px", borderRadius: "8px", border: "1px solid #ccc", background: "#fff", cursor: "pointer", color: "#333" }}
              >
                Tutup
              </button>
              <button 
                className="btn btn-primary" 
                onClick={() => window.print()}
                style={{ padding: "8px 16px", borderRadius: "8px", background: "var(--primary)", color: "#fff", border: "none", cursor: "pointer", display: "inline-flex", alignItems: "center", gap: "6px" }}
              >
                <PrintIcon size={16} />
                Cetak Kwitansi
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
