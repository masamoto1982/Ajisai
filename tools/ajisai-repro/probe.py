import json,subprocess,sys
BIN="/home/user/Ajisai/rust/target/release/ajisai"
def run(src):
    open("/tmp/_p.ajisai","w").write(src)
    r=subprocess.run([BIN,"run","/tmp/_p.ajisai","--json"],capture_output=True,text=True)
    try: d=json.loads(r.stdout)
    except: return {"status":"crash","raw":r.stdout[:200]+r.stderr[:200]}
    out={"status":d["status"]}
    if d["status"]=="ok":
        out["stack"]=d.get("stackDisplay")
        out["output"]=d.get("output")
    else:
        ai=d.get("aiDiagnostic") or {}
        out["kind"]=ai.get("kind")
    return out
if __name__=="__main__":
    for s in sys.argv[1:]:
        print(repr(s),"->",run(s))
