use std::fmt;
use std::iter;

#[derive(Clone, Copy)]
pub struct DomainName<'a>(&'a [u8]);

impl DomainName<'_> {
    pub fn parts(&self) -> impl Iterator<Item = &'_ str> {
        let mut buffer = self.0;
        iter::from_fn(move || {
            if buffer.is_empty() {
                None
            } else {
                let len = 1 + buffer[0] as usize;
                let slice = &buffer[1..len];
                buffer = &buffer[len..];
                Some(std::str::from_utf8(slice).unwrap())
            }
        })
    }

    fn len(buffer: &[u8]) -> usize {
        println!("{:?}", buffer);
        let mut len = 0;
        while buffer[len] != 0 {
            len += 1 + buffer[len] as usize
        }
        len + 1
    }
}

impl fmt::Debug for DomainName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for part in self.parts() {
            f.write_str(part)?;
            f.write_str(".")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Question<'a> {
    pub name: DomainName<'a>,
    pub qtype: u16,
    pub class: u16,
}

impl Question<'_> {
    fn parse(buffer: &[u8]) -> (Question<'_>, usize) {
        let name_end = DomainName::len(buffer);
        let qtype = u16::from_be_bytes([buffer[name_end], buffer[name_end + 1]]);
        let class = u16::from_be_bytes([buffer[name_end + 2], buffer[name_end + 3]]);
        (
            Question {
                name: DomainName(&buffer[0..name_end]),
                qtype,
                class,
            },
            name_end + 4,
        )
    }

    fn len(buffer: &[u8]) -> usize {
        DomainName::len(buffer) + 4
    }
}

pub struct Resource<'a> {
    pub name: DomainName<'a>,
    pub rtype: u16,
    pub class: u16,
    pub ttl: u32,
    pub data: &'a [u8],
}

#[derive(Debug)]
pub struct Header {
    pub id: u16,
    pub query: bool,
    pub opcode: u8,
    pub authoritative_answer: bool,
    pub truncation: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub rcode: u8,
}

#[derive(Debug)]
pub struct Resources<'a>(&'a [u8]);

pub struct Questions<'a>(&'a [u8]);

impl Questions<'_> {
    pub fn iter(&self) -> impl Iterator<Item = Question<'_>> {
        let mut buffer = self.0;
        iter::from_fn(move || {
            if buffer.is_empty() {
                None
            } else {
                let (question, len) = Question::parse(buffer);
                buffer = &buffer[len..];
                Some(question)
            }
        })
    }
}

impl fmt::Debug for Questions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[derive(Debug)]
pub struct ParsedDns<'a> {
    pub header: Header,
    pub questions: Questions<'a>,
    pub answers: Resources<'a>,
    pub authority: Resources<'a>,
    pub additional: Resources<'a>,
}

pub fn parse(buffer: &[u8]) -> ParsedDns<'_> {
    println!("{:?}", buffer);
    let id = u16::from_be_bytes([buffer[0], buffer[1]]);
    let flags = u16::from_be_bytes([buffer[2], buffer[3]]);
    let query = (flags >> 15) == 0;
    let opcode = ((flags >> 11) & 0xF) as u8;
    let authoritative_answer = (flags >> 10) == 1;
    let truncation = (flags >> 9) == 1;
    let recursion_desired = (flags >> 8) == 1;
    let recursion_available = (flags >> 7) == 1;
    let rcode = (flags & 0xF) as u8;
    let qd_count = u16::from_be_bytes([buffer[4], buffer[5]]);
    let an_count = u16::from_be_bytes([buffer[6], buffer[7]]);
    let ns_count = u16::from_be_bytes([buffer[8], buffer[9]]);
    let ar_count = u16::from_be_bytes([buffer[10], buffer[11]]);

    let questions_start = &buffer[12..];
    let mut questions_len = 0;
    for _ in 0..qd_count {
        questions_len = Question::len(&questions_start[questions_len..]);
        break
    }
    let questions = Questions(&questions_start[0..questions_len]);

    ParsedDns {
        header: Header {
            id,
            query,
            opcode,
            authoritative_answer,
            truncation,
            recursion_desired,
            recursion_available,
            rcode,
        },
        questions,
        answers: Resources(&[]),
        authority: Resources(&[]),
        additional: Resources(&[]),
    }
}

pub fn to_bytes(header: Header, answers: &[Resource]) -> Vec<u8> {
    let mut packet = vec![];
    packet.extend_from_slice(&header.id.to_be_bytes());
    let mut flags: u16 = header.rcode as u16;
    if header.query == false {
        flags |= (1 << 15);
    }
    if header.authoritative_answer {
        flags |= (1 << 10);
    }
    packet.extend_from_slice(&flags.to_be_bytes());
    packet.extend_from_slice(&[0, 0]);
    packet.extend_from_slice(&(answers.len() as u16).to_be_bytes());
    packet.extend_from_slice(&[0, 0]);
    packet.extend_from_slice(&[0, 0]);
    for answer in answers {
        packet.extend_from_slice(answer.name.0);
        packet.extend_from_slice(&answer.rtype.to_be_bytes());
        packet.extend_from_slice(&answer.class.to_be_bytes());
        packet.extend_from_slice(&answer.ttl.to_be_bytes());
        packet.extend_from_slice(&(answer.data.len() as u16).to_be_bytes());
        packet.extend_from_slice(answer.data);
    }
    packet
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse() {
        let buffer = [0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 7, 108, 105, 99, 111, 114, 110, 101, 5, 108, 111, 99, 97, 108, 0, 0, 1, 0, 1, 192, 12, 0, 28, 0, 1];
        let result = super::parse(&buffer);
        println!("{:?}", result);
    }
}
