# Email Collector

The _Email Collector_ receives a [MIME email message](https://en.wikipedia.org/wiki/MIME) as
input, parses it and produces a Tornado Event.


## How it works

When the _Email Collector_ receives a valid [MIME email message](https://en.wikipedia.org/wiki/MIME) 
as input, it parses it and produces a Tornado Event with the extracted data.

For example, given the following input:
```
Subject: This is a test email
Content-Type: multipart/alternative; boundary=foobar
Date: Sun, 02 Oct 2016 07:06:22 -0700 (PDT)

--foobar
Content-Type: text/plain; charset=utf-8
Content-Transfer-Encoding: quoted-printable

This is the plaintext version, in utf-8. Proof by Euro: =E2=82=AC
--foobar
Content-Type: text/html
Content-Transfer-Encoding: base64

PGh0bWw+PGJvZHk+VGhpcyBpcyB0aGUgPGI+SFRNTDwvYj4gdmVyc2lvbiwgaW4g 
dXMtYXNjaWkuIFByb29mIGJ5IEV1cm86ICZldXJvOzwvYm9keT48L2h0bWw+Cg== 
--foobar--
```

it will generate this Event:

```json
{
  "type": "email",
  "created_ms": 1554130814854,
  "payload": {
    "date": 1475417182,
    "subject": "This is a test email",
    "to": "",
    "from": "",
    "cc": "",
    "body": "This is the plaintext version, in utf-8. Proof by Euro: €",
    "attachments": []
  }
}
```

In case of attachments, if the attachment is a text file, it will be included in the produced
Event in plain text, otherwise, it will be encoded in base64. 

For example, from this email with attachments:
```mime
From: "Mr.Francesco.Cina" <mr.francesco.cina@gmail.com>
Subject: Test for Mail collector - with attachments
To: "Groeber, Benjamin" <Benjamin.Groeber@wuerth-phoenix.com>,
 francesco cina <mr.francesco.cina@gmail.com>
Cc: Thomas.Forrer@wuerth-phoenix.com, mr.francesco.cina@gmail.com
Date: Sun, 02 Oct 2016 07:06:22 -0700 (PDT)
MIME-Version: 1.0
Content-Type: multipart/mixed;
 boundary="------------E5401F4DD68F2F7A872C2A83"
Content-Language: en-US

This is a multi-part message in MIME format.
--------------E5401F4DD68F2F7A872C2A83
Content-Type: text/html; charset=utf-8
Content-Transfer-Encoding: 7bit

<html>Test for Mail collector with attachments</html>

--------------E5401F4DD68F2F7A872C2A83
Content-Type: application/pdf;
 name="sample.pdf"
Content-Transfer-Encoding: base64
Content-Disposition: attachment;
 filename="sample.pdf"

JVBERi0xLjMNCiXi48/TDQoNCjEgMCBvYmoNCjw8DQovVHlwZSAvQ2F0YWxvZw0KT0YNCg==

--------------E5401F4DD68F2F7A872C2A83
Content-Type: text/plain; charset=UTF-8;
 name="sample.txt"
Content-Transfer-Encoding: base64
Content-Disposition: attachment;
 filename="sample.txt"

dHh0IGZpbGUgY29udGV4dCBmb3IgZW1haWwgY29sbGVjdG9yCjEyMzQ1Njc4OTA5ODc2NTQz
MjEK
--------------E5401F4DD68F2F7A872C2A83--

```

it will generated this Event:
```json
{
  "type": "email",
  "created_ms": 1554130814854,
  "payload": {
    "date": 1475417182,
    "subject": "Test for Mail collector - with attachments",
    "to": "\"Groeber, Benjamin\" <Benjamin.Groeber@wuerth-phoenix.com>, francesco cina <mr.francesco.cina@gmail.com>",
    "from": "\"Mr.Francesco.Cina\" <mr.francesco.cina@gmail.com>",
    "cc": "Thomas.Forrer@wuerth-phoenix.com, mr.francesco.cina@gmail.com",
    "body": "<html>Test for Mail collector with attachments</html>",
    "attachments": [
      {
        "filename": "sample.pdf",
        "mime_type": "application/pdf",
        "encoding": "base64",
        "content": "JVBERi0xLjMNCiXi48/TDQoNCjEgMCBvYmoNCjw8DQovVHlwZSAvQ2F0YWxvZw0KT0YNCg=="
      },
      {
        "filename": "sample.txt",
        "mime_type": "text/plain",
        "encoding": "plaintext",
        "content": "txt file context for email collector\n1234567890987654321\n"
      }
    ]
  }
}
```

In the Tornado Event, the _filename_ and *mime_type* properties of each attachment 
are the values extracted from the incoming email. 

Instead, the _encoding_ property refers to how the _content_ is encoded in the Event itself.
It can be of two types:
- __plaintext__: the content is included in plain text
- __base64__: the content is encoded in base64

## Particular cases
The email collector follows these rules to generate the Tornado Event: 
- if more than a body is present in the email or its subparts, 
  the first usable body found is used, the others will be ignored
- Content Dispositions different from _Inline_ and _Attachment_ are ignored
- Content Dispositions of type _Inline_ are processed only if the content type is _text/*_
- The email subparts are not scanned recursively; this involves that only the subparts at
  the root level are evaluated
