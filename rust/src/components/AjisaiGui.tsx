import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

// --- データ型定義 ---

// (Name, Signature)
type OperatorEntry = [string, string];
// (Name, Content)
type OperandEntry = [string, string];

// --- スタイル ---
const styles: { [key: string]: React.CSSProperties } = {
  container: {
    display: 'flex',
    height: '100vh',
    fontFamily: 'sans-serif',
  },
  leftPanel: {
    width: '40%',
    display: 'flex',
    flexDirection: 'column',
    borderRight: '1px solid #ccc',
  },
  rightPanel: {
    width: '60%',
    display: 'flex',
    flexDirection: 'column',
  },
  panel: {
    flex: 1,
    overflowY: 'auto',
    padding: '10px',
  },
  opPanel: {
    flex: 1,
    borderBottom: '1px solid #ccc',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
  },
  th: {
    borderBottom: '1px solid #333',
    textAlign: 'left',
    padding: '4px',
    backgroundColor: '#f4f4f4',
  },
  td: {
    borderBottom: '1px solid #eee',
    padding: '4px',
    fontFamily: 'monospace',
  },
  console: {
    flex: 1,
    padding: '10px',
    fontFamily: 'monospace',
    backgroundColor: '#282c34',
    color: '#abb2bf',
    overflowY: 'auto',
  },
  input: {
    borderTop: '1px solid #ccc',
    padding: '10px',
    width: '100%',
    boxSizing: 'border-box',
    fontFamily: 'monospace',
    fontSize: '1em',
  },
};

// --- テーブルコンポーネント ---

interface DataTableProps {
  title: string;
  headers: [string, string];
  data: [string, string][];
}

const DataTable: React.FC<DataTableProps> = ({ title, headers, data }) => (
  <div style={styles.panel}>
    <h3>{title}</h3>
    <table style={styles.table}>
      <thead>
        <tr>
          <th style={{...styles.th, width: '30%'}}>{headers[0]}</th>
          <th style={styles.th}>{headers[1]}</th>
        </tr>
      </thead>
      <tbody>
        {data.map(([name, content]) => (
          <tr key={name}>
            <td style={styles.td}>{name}</td>
            <td style={styles.td}>{content}</td>
          </tr>
        ))}
      </tbody>
    </table>
  </div>
);

// --- メインGUIコンポーネント ---

export const AjisaiGui: React.FC = () => {
  const [operators, setOperators] = useState<OperatorEntry[]>([]);
  const [operands, setOperands] = useState<OperandEntry[]>([]);
  const [consoleOut, setConsoleOut] = useState<string[]>([]);
  const [input, setInput] = useState('');

  // データの取得
  const refreshOperators = async () => {
    const ops: OperatorEntry[] = await invoke('get_operators');
    setOperators(ops);
  };

  const refreshOperands = async () => {
    const ops: OperandEntry[] = await invoke('get_operands');
    setOperands(ops);
  };

  // 初期ロード
  useEffect(() => {
    refreshOperators();
    refreshOperands();

    // バックエンドからのイベントをリッスン
    const unlistenOperand = listen('operand-updated', (event) => {
      // (name, content) のタプルが payload
      const [name, content] = event.payload as [string, string];
      setOperands(prev => {
        const index = prev.findIndex(o => o[0] === name);
        if (index > -1) {
          // 既存のものを更新
          const newState = [...prev];
          newState[index] = [name, content];
          return newState;
        } else {
          // 新しいものを追加
          return [...prev, [name, content]].sort((a,b) => a[0].localeCompare(b[0]));
        }
      });
    });

    const unlistenConsole = listen('console-output', (event) => {
      setConsoleOut(prev => [event.payload as string, ...prev]);
    });

    return () => {
      unlistenOperand.then(f => f());
      unlistenConsole.then(f => f());
    };
  }, []);

  // コマンド実行
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (input.trim() === '') return;

    setConsoleOut(prev => [`> ${input}`, ...prev]); // コマンドをエコー
    
    try {
      await invoke('eval_code', { code: input });
      // 実行が成功したら、Operandを更新
      // (注: `operand-updated` イベントが発行されるので、手動refreshは不要かも)
      // refreshOperands(); 
    } catch (error) {
      setConsoleOut(prev => [`Error: ${error}`, ...prev]);
    }
    setInput('');
  };

  return (
    <div style={styles.container}>
      <div style={styles.leftPanel}>
        <div style={{...styles.panel, ...styles.opPanel}}>
          <DataTable
            title="Operator (辞書)"
            headers={['Name', 'Content (Signature)']}
            data={operators}
          />
        </div>
        <div style={styles.panel}>
          <DataTable
            title="Operand (領域)"
            headers={['Name', 'Content (VStack)']}
            data={operands}
          />
        </div>
      </div>
      <div style={styles.rightPanel}>
        <div style={styles.console}>
          {consoleOut.map((line, i) => (
            <div key={i}>{line}</div>
          ))}
        </div>
        <form onSubmit={handleSubmit}>
          <input
            style={styles.input}
            type="text"
            placeholder="[1/2 1/3] -> A"
            value={input}
            onChange={(e) => setInput(e.target.value)}
          />
        </form>
      </div>
    </div>
  );
};
