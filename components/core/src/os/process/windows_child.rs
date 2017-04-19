#[macro_use]
extern crate lazy_static;

use std::sync::Mutex;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::ptr;
use rand::Rng;
use std::io::Error;
use std::ffi::{OsStr, OsString};
use std::mem;
use std::process::*;
use winapi::winbase;
use std::os::windows::ffi::OsStrExt;

lazy_static! {
    static ref CREATE_PROCESS_LOCK: Mutex<()> = Mutex::new(());
}

const INVALID_HANDLE_VALUE: winapi::HANDLE = !0 as winapi::HANDLE;
const CREATE_UNICODE_ENVIRONMENT: winapi::DWORD = 0x00000400;
const HANDLE_FLAG_INHERIT: winapi::DWORD = 0x00000001;

// Note that these are not actually HANDLEs, just values to pass to GetStdHandle
const STD_OUTPUT_HANDLE: winapi::DWORD = -11i32 as winapi::DWORD;
const STD_ERROR_HANDLE: winapi::DWORD = -12i32 as winapi::DWORD;

fn zeroed_startupinfo() -> winapi::STARTUPINFOW {
    winapi::STARTUPINFOW {
        cb: 0,
        lpReserved: ptr::null_mut(),
        lpDesktop: ptr::null_mut(),
        lpTitle: ptr::null_mut(),
        dwX: 0,
        dwY: 0,
        dwXSize: 0,
        dwYSize: 0,
        dwXCountChars: 0,
        dwYCountChars: 0,
        dwFillAttribute: 0,
        dwFlags: 0,
        wShowWindow: 0,
        cbReserved2: 0,
        lpReserved2: ptr::null_mut(),
        hStdInput: INVALID_HANDLE_VALUE,
        hStdOutput: INVALID_HANDLE_VALUE,
        hStdError: INVALID_HANDLE_VALUE,
    }
}

fn zeroed_process_information() -> winapi::PROCESS_INFORMATION {
    winapi::PROCESS_INFORMATION {
        hProcess: ptr::null_mut(),
        hThread: ptr::null_mut(),
        dwProcessId: 0,
        dwThreadId: 0
    }
}

// Produces a wide string *without terminating null*; returns an error if
// `prog` or any of the `args` contain a nul.
fn make_command_line(prog: &OsStr, args: &[&str]) -> io::Result<Vec<u16>> {
    // Encode the command and arguments in a command line string such
    // that the spawned process may recover them using CommandLineToArgvW.
    let mut cmd: Vec<u16> = Vec::new();
    append_arg(&mut cmd, prog)?;
    for arg in args {
        cmd.push(' ' as u16);
        append_arg(&mut cmd, OsStr::new(arg))?;
    }
    return Ok(cmd);

    fn append_arg(cmd: &mut Vec<u16>, arg: &OsStr) -> io::Result<()> {
        // If an argument has 0 characters then we need to quote it to ensure
        // that it actually gets passed through on the command line or otherwise
        // it will be dropped entirely when parsed on the other end.
        let arg_bytes = &arg.to_str().unwrap().as_bytes();
        let quote = arg_bytes.iter().any(|c| *c == b' ' || *c == b'\t')
            || arg_bytes.is_empty();
        if quote {
            cmd.push('"' as u16);
        }

        let mut iter = arg.encode_wide();
        let mut backslashes: usize = 0;
        while let Some(x) = iter.next() {
            if x == '\\' as u16 {
                backslashes += 1;
            } else {
                if x == '"' as u16 {
                    // Add n+1 backslashes to total 2n+1 before internal '"'.
                    for _ in 0..(backslashes+1) {
                        cmd.push('\\' as u16);
                    }
                }
                backslashes = 0;
            }
            cmd.push(x);
        }

        if quote {
            // Add n backslashes to total 2n before ending '"'.
            for _ in 0..backslashes {
                cmd.push('\\' as u16);
            }
            cmd.push('"' as u16);
        }
        Ok(())
    }
}

fn to_u16s<S: AsRef<OsStr>>(s: S) -> io::Result<Vec<u16>> {
    fn inner(s: &OsStr) -> io::Result<Vec<u16>> {
        let mut maybe_result: Vec<u16> = s.encode_wide().collect();
        if maybe_result.iter().any(|&u| u == 0) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput,
                                      "strings passed to WinAPI cannot contain NULs"));
        }
        maybe_result.push(0);
        Ok(maybe_result)
    }
    inner(s.as_ref())
}

fn null_stdio_handle() -> winapi::HANDLE {
    let size = mem::size_of::<winapi::SECURITY_ATTRIBUTES>();
    let mut sa = winapi::SECURITY_ATTRIBUTES {
        nLength: size as winapi::DWORD,
        lpSecurityDescriptor: ptr::null_mut(),
        bInheritHandle: 1,
    };
    let mut opts = OpenOptions::new();
    opts.read(true);
    opts.write(false);
    opts.security_attributes(&mut sa);
    File::open(Path::new("NUL"), &opts).map(|file| {
        file.into_handle()
    }).unwrap()
}

fn stdio_piped_handle(stdio_id: winapi::DWORD, pipe: &mut Option<AnonPipe>) -> io::Result<winapi::HANDLE> {
    let (reader, writer) = anon_pipe()?;
    let (ours, theirs) = if stdio_id == winapi::STD_INPUT_HANDLE {
        (writer, reader)
    } else {
        (reader, writer)
    };
    *pipe = Some(ours);
    unsafe {
        let ret = kernel32::SetHandleInformation(*theirs.handle(),
                                HANDLE_FLAG_INHERIT,
                                HANDLE_FLAG_INHERIT);
        if ret == 0 {
            panic!("Failed to set handle info: {}", Error::last_os_error());
        }

    };
    Ok(theirs.into_handle())
}

pub struct AnonPipe {
    inner: winapi::HANDLE,
}

pub fn anon_pipe() -> io::Result<(AnonPipe, AnonPipe)> {
    // Note that we specifically do *not* use `CreatePipe` here because
    // unfortunately the anonymous pipes returned do not support overlapped
    // operations.
    //
    // Instead, we create a "hopefully unique" name and create a named pipe
    // which has overlapped operations enabled.
    //
    // Once we do this, we connect do it as usual via `CreateFileW`, and then we
    // return those reader/writer halves.
    unsafe {
        let reader;
        let mut name;
        let mut tries = 0;
        let mut reject_remote_clients_flag = winapi::PIPE_REJECT_REMOTE_CLIENTS;
        loop {
            tries += 1;
            let key: u64 = rand::thread_rng().gen();
            name = format!(r"\\.\pipe\__rust_anonymous_pipe1__.{}.{}",
                           kernel32::GetCurrentProcessId(),
                           key);
            let wide_name = OsStr::new(&name)
                                  .encode_wide()
                                  .chain(Some(0))
                                  .collect::<Vec<_>>();

            let handle = kernel32::CreateNamedPipeW(wide_name.as_ptr(),
                                             winapi::PIPE_ACCESS_INBOUND |
                                             winapi::FILE_FLAG_FIRST_PIPE_INSTANCE |
                                             winapi::FILE_FLAG_OVERLAPPED,
                                             winapi::PIPE_TYPE_BYTE |
                                             winapi::PIPE_READMODE_BYTE |
                                             winapi::PIPE_WAIT |
                                             reject_remote_clients_flag,
                                             1,
                                             4096,
                                             4096,
                                             0,
                                             ptr::null_mut());

            // We pass the FILE_FLAG_FIRST_PIPE_INSTANCE flag above, and we're
            // also just doing a best effort at selecting a unique name. If
            // ERROR_ACCESS_DENIED is returned then it could mean that we
            // accidentally conflicted with an already existing pipe, so we try
            // again.
            //
            // Don't try again too much though as this could also perhaps be a
            // legit error.
            // If ERROR_INVALID_PARAMETER is returned, this probably means we're
            // running on pre-Vista version where PIPE_REJECT_REMOTE_CLIENTS is
            // not supported, so we continue retrying without it. This implies
            // reduced security on Windows versions older than Vista by allowing
            // connections to this pipe from remote machines.
            // Proper fix would increase the number of FFI imports and introduce
            // significant amount of Windows XP specific code with no clean
            // testing strategy
            // for more info see https://github.com/rust-lang/rust/pull/37677
            if handle == winapi::INVALID_HANDLE_VALUE {
                let err = io::Error::last_os_error();
                let raw_os_err = err.raw_os_error();
                if tries < 10 {
                    if raw_os_err == Some(winapi::ERROR_ACCESS_DENIED as i32) {
                        continue
                    } else if reject_remote_clients_flag != 0 &&
                        raw_os_err == Some(winapi::ERROR_INVALID_PARAMETER as i32) {
                        reject_remote_clients_flag = 0;
                        tries -= 1;
                        continue
                    }
                }
                return Err(err)
            }
            reader = handle;
            break
        }

        // Connect to the named pipe we just created in write-only mode (also
        // overlapped for async I/O below).
        let mut opts = OpenOptions::new();
        opts.write(true);
        opts.read(false);
        opts.share_mode(0);
        opts.attributes(winapi::FILE_FLAG_OVERLAPPED);
        let writer = File::open(Path::new(&name), &opts)?;
        let writer = AnonPipe { inner: writer.into_handle() };

        Ok((AnonPipe { inner: reader }, AnonPipe { inner: writer.into_handle() }))
    }
}

impl AnonPipe {
    pub fn handle(&self) -> &winapi::HANDLE { &self.inner }
    pub fn into_handle(self) -> winapi::HANDLE { self.inner }
}

struct StdioPipes {
    pub stdin: Option<AnonPipe>,
    pub stdout: Option<AnonPipe>,
    pub stderr: Option<AnonPipe>,
}

#[derive(Clone)]
pub struct OpenOptions {
    // generic
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
    // system-specific
    custom_flags: u32,
    access_mode: Option<winapi::DWORD>,
    attributes: winapi::DWORD,
    share_mode: winapi::DWORD,
    security_qos_flags: winapi::DWORD,
    security_attributes: usize, // FIXME: should be a reference
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            // generic
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
            // system-specific
            custom_flags: 0,
            access_mode: None,
            share_mode: winapi::FILE_SHARE_READ | winapi::FILE_SHARE_WRITE | winapi::FILE_SHARE_DELETE,
            attributes: 0,
            security_qos_flags: 0,
            security_attributes: 0,
        }
    }

    pub fn read(&mut self, read: bool) { self.read = read; }
    pub fn write(&mut self, write: bool) { self.write = write; }
    pub fn append(&mut self, append: bool) { self.append = append; }
    pub fn truncate(&mut self, truncate: bool) { self.truncate = truncate; }
    pub fn create(&mut self, create: bool) { self.create = create; }
    pub fn create_new(&mut self, create_new: bool) { self.create_new = create_new; }

    pub fn custom_flags(&mut self, flags: u32) { self.custom_flags = flags; }
    pub fn access_mode(&mut self, access_mode: u32) { self.access_mode = Some(access_mode); }
    pub fn share_mode(&mut self, share_mode: u32) { self.share_mode = share_mode; }
    pub fn attributes(&mut self, attrs: u32) { self.attributes = attrs; }
    pub fn security_qos_flags(&mut self, flags: u32) { self.security_qos_flags = flags; }
    pub fn security_attributes(&mut self, attrs: winapi::LPSECURITY_ATTRIBUTES) {
        self.security_attributes = attrs as usize;
    }

    fn get_access_mode(&self) -> io::Result<winapi::DWORD> {
        const ERROR_INVALID_PARAMETER: i32 = 87;

        match (self.read, self.write, self.append, self.access_mode) {
            (_,     _,     _, Some(mode)) => Ok(mode),
            (true,  false, false, None) => Ok(winapi::GENERIC_READ),
            (false, true,  false, None) => Ok(winapi::GENERIC_WRITE),
            (true,  true,  false, None) => Ok(winapi::GENERIC_READ | winapi::GENERIC_WRITE),
            (false, _,     true,  None) => Ok(winapi::FILE_GENERIC_WRITE & !winapi::FILE_WRITE_DATA),
            (true,  _,     true,  None) => Ok(winapi::GENERIC_READ |
                                              (winapi::FILE_GENERIC_WRITE & !winapi::FILE_WRITE_DATA)),
            (false, false, false, None) => Err(Error::from_raw_os_error(ERROR_INVALID_PARAMETER)),
        }
    }

    fn get_creation_mode(&self) -> io::Result<winapi::DWORD> {
        const ERROR_INVALID_PARAMETER: i32 = 87;

        match (self.write, self.append) {
            (true, false) => {}
            (false, false) =>
                if self.truncate || self.create || self.create_new {
                    return Err(Error::from_raw_os_error(ERROR_INVALID_PARAMETER));
                },
            (_, true) =>
                if self.truncate && !self.create_new {
                    return Err(Error::from_raw_os_error(ERROR_INVALID_PARAMETER));
                },
        }

        Ok(match (self.create, self.truncate, self.create_new) {
                (false, false, false) => winapi::OPEN_EXISTING,
                (true,  false, false) => winapi::OPEN_ALWAYS,
                (false, true,  false) => winapi::TRUNCATE_EXISTING,
                (true,  true,  false) => winapi::CREATE_ALWAYS,
                (_,      _,    true)  => winapi::CREATE_NEW,
           })
    }

    fn get_flags_and_attributes(&self) -> winapi::DWORD {
        self.custom_flags |
        self.attributes |
        self.security_qos_flags |
        if self.security_qos_flags != 0 { winapi::SECURITY_SQOS_PRESENT } else { 0 } |
        if self.create_new { winapi::FILE_FLAG_OPEN_REPARSE_POINT } else { 0 }
    }
}

pub struct File { handle: winapi::HANDLE }

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let path = to_u16s(path)?;
        let handle = unsafe {
            kernel32::CreateFileW(path.as_ptr(),
                           opts.get_access_mode()?,
                           opts.share_mode,
                           opts.security_attributes as *mut _,
                           opts.get_creation_mode()?,
                           opts.get_flags_and_attributes(),
                           ptr::null_mut())
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(Error::last_os_error())
        } else {
            Ok(File { handle: handle })
        }
    }

    pub fn into_handle(self) -> winapi::HANDLE { self.handle }
}

pub struct HabChild {
    handle: winapi::HANDLE
}

impl HabChild {
  pub fn id(&self) -> u32 {
      unsafe {
          kernel32::GetProcessId(self.handle) as u32
      }
  }

  pub fn spawn_child(program: &str, args: Vec<&str>) -> HabChild {
        let program_path = match env::var_os("PATH") {
          Some(paths) => {
              let mut pp = OsString::new();
              for path in env::split_paths(&paths) {
                let path = path.join(program).with_extension(env::consts::EXE_EXTENSION);
                if fs::metadata(&path).is_ok() {
                    pp = path.into_os_string();
                    break;
                }
              }
              Some(pp)
          }
          None => None
        };

        let mut si = zeroed_startupinfo();
        si.cb = mem::size_of::<winapi::STARTUPINFOW>() as winapi::DWORD;
        si.dwFlags = winbase::STARTF_USESTDHANDLES;

        let program_path = program_path.unwrap_or(OsStr::new(program).to_os_string());
        let mut cmd_str = make_command_line(&program_path, &args).unwrap();
        cmd_str.push(0); // add null terminator

        let mut pi = zeroed_process_information();

        // Prepare all stdio handles to be inherited by the child. This
        // currently involves duplicating any existing ones with the ability to
        // be inherited by child processes. Note, however, that once an
        // inheritable handle is created, *any* spawned child will inherit that
        // handle. We only want our own child to inherit this handle, so we wrap
        // the remaining portion of this spawn in a mutex.
        //
        // For more information, msdn also has an article about this race:
        // http://support.microsoft.com/kb/315939
        CREATE_PROCESS_LOCK.lock().unwrap();

        let mut pipes = StdioPipes {
            stdin: None,
            stdout: None,
            stderr: None,
        };

        let stdin = null_stdio_handle();
        let stdout = stdio_piped_handle(STD_OUTPUT_HANDLE, &mut pipes.stdout).unwrap();
        let stderr = stdio_piped_handle(STD_ERROR_HANDLE, &mut pipes.stderr).unwrap();
        si.hStdInput = stdin;
        si.hStdOutput = stdout;
        si.hStdError = stderr;

        unsafe {
            let ret = kernel32::CreateProcessW(ptr::null(),
                    cmd_str.as_mut_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    winapi::TRUE, CREATE_UNICODE_ENVIRONMENT, ptr::null_mut(), ptr::null(),
                    &mut si, &mut pi);
            if ret == 0 {
                panic!("Failed to start {}: {}", program, Error::last_os_error());
            }
        }

        // We close the thread handle because we don't care about keeping
        // the thread id valid, and we aren't keeping the thread handle
        // around to be able to close it later.
        unsafe { kernel32::CloseHandle(pi.hThread) };

        HabChild { handle: pi.hProcess }
  }

  pub fn exit_state(&self) -> u32 {
      unsafe {
          let mut exit_status: u32 = 0;
          let ret = kernel32::GetExitCodeProcess(self.handle, &mut exit_status as winapi::LPDWORD);
          if ret == 0 {
              panic!("Failed to retrieve Exit Code for {}: {}", self.id(), Error::last_os_error());
          }

          if exit_status == 259 { exit_status = 0 };
          exit_status
      }
  }

  pub fn get_handle(&self) {
      unsafe {
          let pid = kernel32::GetProcessId(self.handle);

          let proc_handle = kernel32::OpenProcess(winapi::PROCESS_QUERY_LIMITED_INFORMATION, winapi::FALSE, pid);
          if proc_handle == ptr::null_mut() {
              panic!("Failed to retrieve Process Handle for {}: {}", pid, Error::last_os_error());
          }

          let pid2 = kernel32::GetProcessId(proc_handle);
          println!("got second handle to pid {}", pid2);
      }
  }
}

impl Drop for HabChild {
    fn drop(&mut self) {
        unsafe { let _ = kernel32::CloseHandle(self.handle); }
    }
}

fn main() {
  let child = HabChild::spawn_child("notepad.exe", Vec::new());

  println!("spawned notepad as pid {}", child.id());
  child.get_handle();

  // println!("Kill it (or dont)!");

  // let mut fuck = String::new();
  // io::stdin().read_line(&mut fuck);

  // println!("notepad exited with {}", child.exit_state());
}
